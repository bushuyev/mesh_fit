use std::path::PathBuf;
use anyhow::{bail, Context, Result};
use image::{DynamicImage, RgbImage};
use ndarray::{Array4, ArrayView1, Axis, Ix3};
use ort::{session::Session, value::TensorRef};

const INPUT_W: usize = 288;
const INPUT_H: usize = 384;
const INPUT_Z: usize = 288;
const SIMCC_SPLIT_RATIO: f32 = 2.0;
const Z_RANGE: f32 = 2.1744869;

// MMPose PoseDataPreprocessor mean/std for RTMW3D config.
const MEAN: [f32; 3] = [123.675, 116.28, 103.53];
const STD: [f32; 3] = [58.395, 57.12, 57.375];

// MMPose GetBBoxCenterScale default padding.
const BBOX_PADDING: f32 = 1.25;

#[derive(Clone, Copy, Debug)]
pub struct BBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Keypoint3D {
    pub x: f32,      // original image x
    pub y: f32,      // original image y
    pub z_rel: f32,  // root-relative z from codec decode
    pub score: f32,  // SimCC score (not a calibrated probability)
}

#[derive(Clone, Copy, Debug)]
struct CropTransform {
    center_x: f32,
    center_y: f32,
    scale_w: f32,
    scale_h: f32,
}

pub struct Rtmw3d {
    session: Session,
}

impl Rtmw3d {
    pub fn new(model_path: PathBuf) -> Result<Self> {
        let session = Session::builder()?
            .commit_from_file(&model_path)
            .with_context(|| format!("failed to load ONNX model from {model_path:?}"))?;
        Ok(Self { session })
    }

    /// Run RTMW3D for one detected person bbox.
    pub fn infer_bbox(&mut self, image: &DynamicImage, bbox: BBox) -> Result<Vec<Keypoint3D>> {
        let rgb = image.to_rgb8();
        let (input, tfm) = preprocess_person(&rgb, bbox)?;

        let outputs = self
            .session
            .run(ort::inputs![TensorRef::from_array_view(&input)?])?;

        // Exported RTMW3D ONNX is expected to return:
        //   x: [1, 133, 576]
        //   y: [1, 133, 768]
        //   z: [1, 133, 576]
        let pred_x = outputs[0]
            .try_extract_array::<f32>()?
            .into_dimensionality::<Ix3>()
            .context("output 0 is not rank-3 f32")?;
        let pred_y = outputs[1]
            .try_extract_array::<f32>()?
            .into_dimensionality::<Ix3>()
            .context("output 1 is not rank-3 f32")?;
        let pred_z = outputs[2]
            .try_extract_array::<f32>()?
            .into_dimensionality::<Ix3>()
            .context("output 2 is not rank-3 f32")?;

        if pred_x.shape() != [1, 133, INPUT_W * 2] {
            bail!("unexpected pred_x shape: {:?}", pred_x.shape());
        }
        if pred_y.shape() != [1, 133, INPUT_H * 2] {
            bail!("unexpected pred_y shape: {:?}", pred_y.shape());
        }
        if pred_z.shape() != [1, 133, INPUT_Z * 2] {
            bail!("unexpected pred_z shape: {:?}", pred_z.shape());
        }

        let x0 = pred_x.index_axis(Axis(0), 0);
        let y0 = pred_y.index_axis(Axis(0), 0);
        let z0 = pred_z.index_axis(Axis(0), 0);

        let mut out = Vec::with_capacity(133);

        for k in 0..x0.len_of(Axis(0)) {
            let (ix, max_x) = argmax_and_max(x0.index_axis(Axis(0), k));
            let (iy, max_y) = argmax_and_max(y0.index_axis(Axis(0), k));
            let (iz, _max_z) = argmax_and_max(z0.index_axis(Axis(0), k));

            // MMPose get_simcc_maximum uses min(max_x, max_y) as score.
            let score = max_x.min(max_y);

            if score <= 0.0 {
                out.push(Keypoint3D {
                    x: f32::NAN,
                    y: f32::NAN,
                    z_rel: f32::NAN,
                    score,
                });
                continue;
            }

            // SimCC index -> crop coordinates.
            let crop_x = ix as f32 / SIMCC_SPLIT_RATIO;
            let crop_y = iy as f32 / SIMCC_SPLIT_RATIO;
            let crop_z = iz as f32 / SIMCC_SPLIT_RATIO;

            // Crop coords -> original image coords.
            let img_x = tfm.center_x
                + (crop_x - (INPUT_W as f32 * 0.5)) * (tfm.scale_w / INPUT_W as f32);
            let img_y = tfm.center_y
                + (crop_y - (INPUT_H as f32 * 0.5)) * (tfm.scale_h / INPUT_H as f32);

            // Decode z the same way as SimCC3DLabel.decode().
            let z_rel = (crop_z / (INPUT_Z as f32 / 2.0) - 1.0) * Z_RANGE;

            out.push(Keypoint3D {
                x: img_x,
                y: img_y,
                z_rel,
                score,
            });
        }

        Ok(out)
    }
}

fn preprocess_person(rgb: &RgbImage, bbox: BBox) -> Result<(Array4<f32>, CropTransform)> {
    let (center_x, center_y, mut scale_w, mut scale_h) = bbox_xyxy_to_center_scale(bbox);

    // Match MMPose TopdownAffine fixed aspect ratio.
    let aspect = INPUT_W as f32 / INPUT_H as f32; // 288 / 384 = 0.75
    if scale_w > scale_h * aspect {
        scale_h = scale_w / aspect;
    } else {
        scale_w = scale_h * aspect;
    }

    let tfm = CropTransform {
        center_x,
        center_y,
        scale_w,
        scale_h,
    };

    let mut input = Array4::<f32>::zeros((1, 3, INPUT_H, INPUT_W));

    for dy in 0..INPUT_H {
        for dx in 0..INPUT_W {
            // This is the inverse mapping for the rotation=0 top-down affine crop.
            let src_x = tfm.center_x
                + (dx as f32 - INPUT_W as f32 * 0.5) * (tfm.scale_w / INPUT_W as f32);
            let src_y = tfm.center_y
                + (dy as f32 - INPUT_H as f32 * 0.5) * (tfm.scale_h / INPUT_H as f32);

            let [r, g, b] = bilinear_sample_rgb(rgb, src_x, src_y);

            // Image is already RGB here, so do not swap channels.
            input[[0, 0, dy, dx]] = (r - MEAN[0]) / STD[0];
            input[[0, 1, dy, dx]] = (g - MEAN[1]) / STD[1];
            input[[0, 2, dy, dx]] = (b - MEAN[2]) / STD[2];
        }
    }

    Ok((input, tfm))
}

fn bbox_xyxy_to_center_scale(b: BBox) -> (f32, f32, f32, f32) {
    let w = (b.x2 - b.x1).max(1.0);
    let h = (b.y2 - b.y1).max(1.0);
    let cx = 0.5 * (b.x1 + b.x2);
    let cy = 0.5 * (b.y1 + b.y2);

    (cx, cy, w * BBOX_PADDING, h * BBOX_PADDING)
}

fn bilinear_sample_rgb(img: &RgbImage, x: f32, y: f32) -> [f32; 3] {
    let w = img.width() as i32;
    let h = img.height() as i32;

    if x < 0.0 || y < 0.0 || x > (w - 1) as f32 || y > (h - 1) as f32 {
        return [0.0, 0.0, 0.0];
    }

    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);

    let dx = x - x0 as f32;
    let dy = y - y0 as f32;

    let p00 = img.get_pixel(x0 as u32, y0 as u32).0;
    let p10 = img.get_pixel(x1 as u32, y0 as u32).0;
    let p01 = img.get_pixel(x0 as u32, y1 as u32).0;
    let p11 = img.get_pixel(x1 as u32, y1 as u32).0;

    let mut out = [0.0f32; 3];
    for c in 0..3 {
        let v00 = p00[c] as f32;
        let v10 = p10[c] as f32;
        let v01 = p01[c] as f32;
        let v11 = p11[c] as f32;

        let top = v00 * (1.0 - dx) + v10 * dx;
        let bot = v01 * (1.0 - dx) + v11 * dx;
        out[c] = top * (1.0 - dy) + bot * dy;
    }
    out
}

fn argmax_and_max(v: ArrayView1<'_, f32>) -> (usize, f32) {
    let mut best_i = 0usize;
    let mut best_v = f32::NEG_INFINITY;
    for (i, &x) in v.iter().enumerate() {
        if x > best_v {
            best_v = x;
            best_i = i;
        }
    }
    (best_i, best_v)
}

// Example usage:
//
pub(crate) fn run_rtm_w3d(model_path: PathBuf, img:&DynamicImage, bbox:BBox) -> Result<()> {
    let mut model = Rtmw3d::new(model_path)?;

    // bbox from RTMDet in original image coordinates
   

    let kpts = model.infer_bbox(&img, bbox)?;
    for (i, kp) in kpts.iter().enumerate().take(10) {
        println!(
            "#{i:03}: x={:.1}, y={:.1}, z_rel={:.4}, score={:.4}",
            kp.x, kp.y, kp.z_rel, kp.score
        );
    }
    Ok(())
}