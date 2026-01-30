#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
import urllib.request

import cv2
import numpy as np
import mediapipe as mp

# Official MediaPipe model download endpoints
POSE_FULL_URL = "https://storage.googleapis.com/mediapipe-models/pose_landmarker/pose_landmarker_full/float16/latest/pose_landmarker_full.task"
SEG_SELFIE_URL = "https://storage.googleapis.com/mediapipe-models/image_segmenter/selfie_segmenter/float16/latest/selfie_segmenter.tflite"


def _mkdir(p: Path) -> None:
    p.mkdir(parents=True, exist_ok=True)


def download_if_missing(url: str, dst: Path) -> Path:
    _mkdir(dst.parent)
    if dst.exists() and dst.stat().st_size > 0:
        return dst
    print(f"[download] {url} -> {dst}")
    urllib.request.urlretrieve(url, dst)
    if not dst.exists() or dst.stat().st_size == 0:
        raise RuntimeError(f"Download failed or empty file: {dst}")
    return dst


def list_images(indir: Path):
    exts = {".jpg", ".jpeg", ".png", ".webp", ".bmp"}
    files = [p for p in indir.iterdir() if p.is_file() and p.suffix.lower() in exts]
    files.sort()
    return files


def mp_image_from_bgr(bgr: np.ndarray) -> mp.Image:
    rgb = cv2.cvtColor(bgr, cv2.COLOR_BGR2RGB)
    return mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb)


def mpimage_to_numpy(mpi):
    if mpi is None:
        return None
    if hasattr(mpi, "numpy_view"):
        return mpi.numpy_view()
    return None


def ensure_hw(arr: np.ndarray, name="mask") -> np.ndarray:
    """Convert mask-like arrays to shape [H,W] by squeezing singleton dims."""
    a = np.asarray(arr)
    a = np.squeeze(a)  # kills (H,W,1), (1,H,W), (1,H,W,1), etc.
    if a.ndim == 3:
        # If still 3D, keep first channel (rare, but safe)
        a = a[..., 0]
    if a.ndim != 2:
        raise ValueError(f"{name} expected 2D HxW after squeeze, got shape={a.shape}, dtype={a.dtype}")
    return a


def get_category_mask(seg_result):
    # MediaPipe API varies a bit; handle both.
    if seg_result is None:
        return None
    if hasattr(seg_result, "category_mask") and seg_result.category_mask is not None:
        return seg_result.category_mask
    if hasattr(seg_result, "category_masks") and seg_result.category_masks:
        return seg_result.category_masks[0]
    return None


def get_pose_landmarks(pose_result):
    if pose_result is None or not hasattr(pose_result, "pose_landmarks"):
        return None
    if not pose_result.pose_landmarks:
        return None
    return pose_result.pose_landmarks[0]  # first detected person


def landmarks_to_pose2d(landmarks, w: int, h: int) -> np.ndarray:
    n = len(landmarks)
    pose2d = np.zeros((n, 3), dtype=np.float32)
    for i, lm in enumerate(landmarks):
        x = float(lm.x) * w
        y = float(lm.y) * h
        vis = float(getattr(lm, "visibility", 1.0))
        pose2d[i] = (x, y, vis)
    return pose2d


def bbox_from_mask(mask01: np.ndarray):
    m = ensure_hw(mask01, "mask01")
    ys, xs = np.where(m > 0)
    if xs.size == 0 or ys.size == 0:
        return None
    x0, x1 = int(xs.min()), int(xs.max()) + 1
    y0, y1 = int(ys.min()), int(ys.max()) + 1
    return x0, y0, x1, y1


def expand_bbox(bbox, w: int, h: int, margin: float):
    x0, y0, x1, y1 = bbox
    bw = x1 - x0
    bh = y1 - y0
    mx = int(round(bw * margin))
    my = int(round(bh * margin))
    x0 = max(0, x0 - mx)
    y0 = max(0, y0 - my)
    x1 = min(w, x1 + mx)
    y1 = min(h, y1 + my)
    return x0, y0, x1, y1


def crop_pad_resize(bgr, mask01, pose2d, bbox, out_size: int):
    """Crop to bbox, pad to square, resize to out_size. Updates pose2d accordingly."""
    H, W = bgr.shape[:2]
    x0, y0, x1, y1 = bbox

    mask01 = ensure_hw(mask01, "mask01").astype(np.uint8)

    crop_bgr = bgr[y0:y1, x0:x1].copy()
    crop_mask = mask01[y0:y1, x0:x1].copy()

    pose2d2 = pose2d.copy()
    pose2d2[:, 0] -= x0
    pose2d2[:, 1] -= y0

    ch, cw = crop_bgr.shape[:2]
    side = max(ch, cw)

    pad_top = (side - ch) // 2
    pad_bottom = side - ch - pad_top
    pad_left = (side - cw) // 2
    pad_right = side - cw - pad_left

    crop_bgr = cv2.copyMakeBorder(
        crop_bgr, pad_top, pad_bottom, pad_left, pad_right,
        borderType=cv2.BORDER_CONSTANT, value=(0, 0, 0)
    )
    crop_mask = cv2.copyMakeBorder(
        crop_mask, pad_top, pad_bottom, pad_left, pad_right,
        borderType=cv2.BORDER_CONSTANT, value=0
    )

    pose2d2[:, 0] += pad_left
    pose2d2[:, 1] += pad_top

    scale = out_size / float(side)
    out_bgr = cv2.resize(crop_bgr, (out_size, out_size), interpolation=cv2.INTER_AREA)
    out_mask = cv2.resize(crop_mask, (out_size, out_size), interpolation=cv2.INTER_NEAREST).astype(np.uint8)

    pose2d2[:, 0] *= scale
    pose2d2[:, 1] *= scale

    meta = {
        "orig_size": [int(W), int(H)],
        "crop_bbox_xyxy": [int(x0), int(y0), int(x1), int(y1)],
        "pad_tblr": [int(pad_top), int(pad_bottom), int(pad_left), int(pad_right)],
        "square_side_before_resize": int(side),
        "out_size": int(out_size),
        "scale": float(scale),
    }
    return out_bgr, out_mask, pose2d2, meta


def signed_distance_field(mask01: np.ndarray) -> np.ndarray:
    """
    SDF with sign suitable for 'penalize outside': negative inside, positive outside.
    """
    m = (ensure_hw(mask01, "mask01") > 0).astype(np.uint8)
    dist_out = cv2.distanceTransform((1 - m).astype(np.uint8), cv2.DIST_L2, 5)
    dist_in  = cv2.distanceTransform(m.astype(np.uint8),       cv2.DIST_L2, 5)
    sdf = (dist_out - dist_in).astype(np.float32)  # <0 inside, >0 outside
    sdf /= float(max(m.shape[0], m.shape[1]))
    return sdf


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--indir", required=True, type=str)
    ap.add_argument("--outdir", required=True, type=str)
    ap.add_argument("--size", type=int, default=1024)
    ap.add_argument("--margin", type=float, default=0.12)
    ap.add_argument("--cache", type=str, default=".cache/mediapipe_models")
    ap.add_argument("--pose_model", type=str, default="pose_landmarker_full.task")
    ap.add_argument("--seg_model", type=str, default="selfie_segmenter.tflite")
    ap.add_argument("--no_seg", action="store_true")
    args = ap.parse_args()

    indir = Path(args.indir)
    outdir = Path(args.outdir)
    cache = Path(args.cache)
    _mkdir(outdir)

    pose_model_path = Path(args.pose_model)
    if not pose_model_path.is_absolute():
        pose_model_path = cache / pose_model_path
    pose_model_path = download_if_missing(POSE_FULL_URL, pose_model_path)

    seg_model_path = None
    if not args.no_seg:
        seg_model_path = Path(args.seg_model)
        if not seg_model_path.is_absolute():
            seg_model_path = cache / seg_model_path
        seg_model_path = download_if_missing(SEG_SELFIE_URL, seg_model_path)

    BaseOptions = mp.tasks.BaseOptions
    VisionRunningMode = mp.tasks.vision.RunningMode

    pose_options = mp.tasks.vision.PoseLandmarkerOptions(
        base_options=BaseOptions(model_asset_path=str(pose_model_path)),
        running_mode=VisionRunningMode.IMAGE,
        num_poses=1,
        output_segmentation_masks=False,
    )

    seg_options = None
    if seg_model_path is not None:
        seg_options = mp.tasks.vision.ImageSegmenterOptions(
            base_options=BaseOptions(model_asset_path=str(seg_model_path)),
            running_mode=VisionRunningMode.IMAGE,
            output_category_mask=True,
            output_confidence_masks=False,
        )

    files = list_images(indir)
    if not files:
        raise SystemExit(f"No images found in {indir}")

    with mp.tasks.vision.PoseLandmarker.create_from_options(pose_options) as poser:
        segger = None
        if seg_options is not None:
            segger = mp.tasks.vision.ImageSegmenter.create_from_options(seg_options)

        try:
            for i, img_path in enumerate(files):
                bgr = cv2.imread(str(img_path), cv2.IMREAD_COLOR)
                if bgr is None:
                    print(f"[warn] failed to read: {img_path}")
                    continue
                H, W = bgr.shape[:2]
                mpimg = mp_image_from_bgr(bgr)

                pose_result = poser.detect(mpimg)
                lms = get_pose_landmarks(pose_result)
                if lms is None:
                    print(f"[warn] no pose found: {img_path.name}")
                    pose2d = np.zeros((33, 3), dtype=np.float32)
                else:
                    pose2d = landmarks_to_pose2d(lms, W, H)

                if segger is not None:
                    seg_result = segger.segment(mpimg)
                    cat_img = get_category_mask(seg_result)
                    cat = mpimage_to_numpy(cat_img)
                    if cat is None:
                        print(f"[warn] no category mask: {img_path.name}")
                        mask01 = np.ones((H, W), dtype=np.uint8)
                    else:
                        cat = ensure_hw(cat, "category_mask")
                        # selfie segmenter: background=0, person=1 (usually),
                        # but be robust if it comes as 0/255.
                        if cat.max() <= 2:
                            mask01 = (cat.astype(np.uint8) == 1).astype(np.uint8)
                        else:
                            mask01 = (cat.astype(np.uint8) > 0).astype(np.uint8)
                else:
                    mask01 = np.ones((H, W), dtype=np.uint8)

                bbox = bbox_from_mask(mask01)
                if bbox is None:
                    bbox = (0, 0, W, H)
                bbox = expand_bbox(bbox, W, H, args.margin)

                out_bgr, out_mask01, out_pose2d, meta = crop_pad_resize(
                    bgr, mask01, pose2d, bbox, args.size
                )
                sdf = signed_distance_field(out_mask01)

                view_dir = outdir / f"{i:03d}"
                _mkdir(view_dir)

                cv2.imwrite(str(view_dir / "rgb.png"), out_bgr)
                cv2.imwrite(str(view_dir / "mask.png"), (out_mask01 * 255).astype(np.uint8))
                np.save(str(view_dir / "pose2d_33.npy"), out_pose2d)
                np.save(str(view_dir / "sdf.npy"), sdf)

                (view_dir / "meta.json").write_text(
                    json.dumps({"src": str(img_path), **meta}, indent=2),
                    encoding="utf-8",
                )

                # quick quality metric:
                vis = out_pose2d[:, 2]
                avg_vis = float(vis.mean()) if vis.size else 0.0
                print(f"[ok] {img_path.name} -> {view_dir.name}  avg_vis={avg_vis:.3f}")
        finally:
            if segger is not None:
                segger.close()


if __name__ == "__main__":
    main()

