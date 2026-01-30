import json, cv2
import mediapipe as mp
import argparse

MP = mp.solutions.pose.PoseLandmark
USE = {
    "nose": MP.NOSE,
    "l_shoulder": MP.LEFT_SHOULDER, "r_shoulder": MP.RIGHT_SHOULDER,
    "l_elbow": MP.LEFT_ELBOW, "r_elbow": MP.RIGHT_ELBOW,
    "l_wrist": MP.LEFT_WRIST, "r_wrist": MP.RIGHT_WRIST,
    "l_hip": MP.LEFT_HIP, "r_hip": MP.RIGHT_HIP,
    "l_knee": MP.LEFT_KNEE, "r_knee": MP.RIGHT_KNEE,
    "l_ankle": MP.LEFT_ANKLE, "r_ankle": MP.RIGHT_ANKLE,
}

def main(img_path, out_path):
    img = cv2.imread(img_path)
    h, w = img.shape[:2]
    rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)

    with mp.solutions.pose.Pose(static_image_mode=True, model_complexity=2) as pose:
        res = pose.process(rgb)

    out = {"image": img_path, "w": w, "h": h, "keypoints": {}}
    if res.pose_landmarks:
        lms = res.pose_landmarks.landmark
        for name, idx in USE.items():
            lm = lms[idx]
            out["keypoints"][name] = {"x": lm.x * w, "y": lm.y * h, "c": float(lm.visibility)}
    with open(out_path, "w") as f:
        json.dump(out, f, indent=2)
    print("wrote", out_path)

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--img", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    main(a.img, a.out)
