use std::ops::{Add, Sub, SubAssign};
use log::debug;

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

impl ArmatureDesc {
    pub fn base_to(&mut self, bone_name:&str) {
        let base_bone = self.bones.iter().find(|bone|bone.name.eq(&bone_name)).expect(&format!("no {bone_name:?} bone")).clone();
        for bone in &mut self.bones {
            bone.tail -= base_bone.tail;
            bone.head -= base_bone.tail;
            println!("ArmatureDesc::base_to: bone={bone:?}");
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Keypoint3D {
    pub x: f32,      // original image x
    pub y: f32,      // original image y
    pub z_rel: f32,  // root-relative z from codec decode
}

impl Sub for Keypoint3D {
    type Output = Keypoint3D;

    fn sub(self, rhs: Self) -> Self::Output {
        Keypoint3D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z_rel: self.z_rel - rhs.z_rel,
        }
    }
}

impl SubAssign for Keypoint3D {
    fn sub_assign(&mut self, rhs: Self) {
        self.x-= rhs.x;
        self.y-= rhs.y;
        self.z_rel-= rhs.z_rel;
    }
}


impl Add for Keypoint3D {
    type Output = Keypoint3D;

    fn add(self, rhs: Self) -> Self::Output {
        Keypoint3D {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z_rel: self.z_rel + rhs.z_rel,
        }
    }
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