#[derive(Debug, Clone, PartialEq)]
pub struct BoneDesc {
    pub name: String,
    pub parent_index: i32,
    pub head: Keypoint3D,
    pub tail: Keypoint3D,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArmatureDesc {
    pub bones: Vec<BoneDesc>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Keypoint3D {
    pub x: f32,      // original image x
    pub y: f32,      // original image y
    pub z_rel: f32,  // root-relative z from codec decode
    // pub score: f32,  // SimCC score (not a calibrated probability)
}

impl From<[f32;3]> for Keypoint3D {
    fn from(value: [f32; 3]) -> Self {
        let [x,y, z_rel] = value;
        Keypoint3D {
            x,
            y,
            z_rel
        }
    }
}

impl From<Keypoint3D> for [f32;3]  {
    fn from(value: Keypoint3D) -> Self {
        let Keypoint3D { x, y, z_rel} = value;
        [x, y, z_rel]
    }
}