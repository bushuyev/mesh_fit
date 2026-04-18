use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use meshfit_shared::armature::{ArmatureDesc, BoneDesc};

unsafe extern "C" {
    fn blender_shim_version_major() -> c_int;
    fn blender_shim_version_minor() -> c_int;
    fn blender_shim_version_patch() -> c_int;
    fn blender_shim_version_string(out: *mut c_char, out_size: c_int) -> c_int;

    fn blender_shim_normalize_vec3(input: *const f32, output: *mut f32) -> f32;
    fn blender_shim_dot_vec3(a: *const f32, b: *const f32) -> f32;

    fn blender_shim_write_armature_desc_to_blend(
        armature: *const BlenderShimArmatureDesc,
        blend_path: *const c_char,
        error_out: *mut c_char,
        error_out_size: c_int,
    ) -> BlenderShimWriteBlendResult;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimBoneFromJointsResult {
    pub head: BlenderShimVec3,
    pub tail: BlenderShimVec3,
    pub direction_unit: BlenderShimVec3,
    pub length: f32,
    pub ok: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneFromJoints {
    pub head: [f32; 3],
    pub tail: [f32; 3],
    pub direction_unit: [f32; 3],
    pub length: f32,
    pub ok: bool,
}


pub fn blender_version() -> (i32, i32, i32) {
    unsafe {
        (
            blender_shim_version_major(),
            blender_shim_version_minor(),
            blender_shim_version_patch(),
        )
    }
}

pub fn blender_version_string() -> String {
    let mut buf = vec![0u8; 64];
    unsafe {
        let written = blender_shim_version_string(buf.as_mut_ptr() as *mut c_char, buf.len() as c_int);
        if written < 0 {
            panic!("blender_shim_version_string failed");
        }
        CStr::from_ptr(buf.as_ptr() as *const c_char)
            .to_string_lossy()
            .into_owned()
    }
}

pub fn normalize_vec3(input: [f32; 3]) -> ([f32; 3], f32) {
    let mut out = [0.0_f32; 3];
    let len = unsafe { blender_shim_normalize_vec3(input.as_ptr(), out.as_mut_ptr()) };
    (out, len)
}

pub fn dot_vec3(a: [f32; 3], b: [f32; 3]) -> f32 {
    unsafe { blender_shim_dot_vec3(a.as_ptr(), b.as_ptr()) }
}

fn to_ffi_vec3(v: [f32; 3]) -> BlenderShimVec3 {
    BlenderShimVec3 {
        x: v[0],
        y: v[1],
        z: v[2],
    }
}


#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointId {
    Pelvis = 0,
    Spine = 1,
    Neck = 2,
    LeftShoulder = 3,
    RightShoulder = 4,
    LeftHip = 5,
    RightHip = 6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimNamedJoint {
    pub joint_id: i32,
    pub position: BlenderShimVec3,
    pub confidence: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimSimpleChainResult {
    pub pelvis_found: i32,
    pub spine_found: i32,
    pub neck_found: i32,

    pub pelvis: BlenderShimVec3,
    pub spine: BlenderShimVec3,
    pub neck: BlenderShimVec3,

    pub pelvis_to_spine: BlenderShimBoneFromJointsResult,
    pub spine_to_neck: BlenderShimBoneFromJointsResult,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamedJoint {
    pub joint_id: JointId,
    pub position: [f32; 3],
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimpleChain {
    pub pelvis_found: bool,
    pub spine_found: bool,
    pub neck_found: bool,

    pub pelvis: [f32; 3],
    pub spine: [f32; 3],
    pub neck: [f32; 3],

    pub pelvis_to_spine: BoneFromJoints,
    pub spine_to_neck: BoneFromJoints,
}


#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimTorsoLandmarksResult {
    pub pelvis_found: i32,
    pub neck_found: i32,
    pub left_shoulder_found: i32,
    pub right_shoulder_found: i32,
    pub left_hip_found: i32,
    pub right_hip_found: i32,

    pub pelvis_center_ok: i32,
    pub shoulder_center_ok: i32,
    pub torso_up_ok: i32,
    pub shoulder_axis_ok: i32,
    pub hip_axis_ok: i32,

    pub pelvis: BlenderShimVec3,
    pub neck: BlenderShimVec3,
    pub left_shoulder: BlenderShimVec3,
    pub right_shoulder: BlenderShimVec3,
    pub left_hip: BlenderShimVec3,
    pub right_hip: BlenderShimVec3,

    pub pelvis_center: BlenderShimVec3,
    pub shoulder_center: BlenderShimVec3,

    pub torso_up: BlenderShimBoneFromJointsResult,
    pub shoulder_axis: BlenderShimBoneFromJointsResult,
    pub hip_axis: BlenderShimBoneFromJointsResult,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TorsoLandmarks {
    pub pelvis_found: bool,
    pub neck_found: bool,
    pub left_shoulder_found: bool,
    pub right_shoulder_found: bool,
    pub left_hip_found: bool,
    pub right_hip_found: bool,

    pub pelvis_center_ok: bool,
    pub shoulder_center_ok: bool,
    pub torso_up_ok: bool,
    pub shoulder_axis_ok: bool,
    pub hip_axis_ok: bool,

    pub pelvis: [f32; 3],
    pub neck: [f32; 3],
    pub left_shoulder: [f32; 3],
    pub right_shoulder: [f32; 3],
    pub left_hip: [f32; 3],
    pub right_hip: [f32; 3],

    pub pelvis_center: [f32; 3],
    pub shoulder_center: [f32; 3],

    pub torso_up: BoneFromJoints,
    pub shoulder_axis: BoneFromJoints,
    pub hip_axis: BoneFromJoints,
}




#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimBasis3 {
    pub x_axis: BlenderShimVec3,
    pub y_axis: BlenderShimVec3,
    pub z_axis: BlenderShimVec3,
    pub ok: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimTorsoFrameResult {
    pub landmarks: BlenderShimTorsoLandmarksResult,
    pub origin: BlenderShimVec3,
    pub basis: BlenderShimBasis3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Basis3 {
    pub x_axis: [f32; 3],
    pub y_axis: [f32; 3],
    pub z_axis: [f32; 3],
    pub ok: bool,
}



#[repr(C)]
#[derive(Debug)]
pub struct BlenderShimBoneDesc {
    pub name: *const c_char,
    pub parent_index: i32,
    pub head: BlenderShimVec3,
    pub tail: BlenderShimVec3,
}

#[repr(C)]
#[derive(Debug)]
pub struct BlenderShimArmatureDesc {
    pub bones: *const BlenderShimBoneDesc,
    pub bone_count: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimArmatureValidationResult {
    pub ok: i32,
    pub has_invalid_parent: i32,
    pub has_degenerate_bone: i32,
    pub first_invalid_bone_index: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmatureValidationResult {
    pub ok: bool,
    pub has_invalid_parent: bool,
    pub has_degenerate_bone: bool,
    pub first_invalid_bone_index: i32,
}

pub fn make_simple_torso_armature() -> ArmatureDesc {
    let pelvis = [1.0, 2.0, 3.0];
    let spine = [1.0, 2.0, 3.0];
    let neck = [1.0, 2.0, 3.0];

    ArmatureDesc {
        bones: vec![
            BoneDesc {
                name: "pelvis".to_string(),
                parent_index: -1,
                head: pelvis.into(),
                tail: spine.into(),
            },
            BoneDesc {
                name: "spine".to_string(),
                parent_index: 0,
                head: spine.into(),
                tail: neck.into(),
            },
        ],
    }
}


pub fn write_armature_desc_to_blend(
    armature: &ArmatureDesc,
    blend_path: &std::path::Path,
) -> Result<(), String> {
    let c_names: Vec<CString> = armature
        .bones
        .iter()
        .map(|b| CString::new(b.name.as_str()).expect("bone name must not contain interior NUL bytes"))
        .collect();

    let ffi_bones: Vec<BlenderShimBoneDesc> = armature
        .bones
        .iter()
        .zip(c_names.iter())
        .map(|(b, name)| BlenderShimBoneDesc {
            name: name.as_ptr(),
            parent_index: b.parent_index,
            head: to_ffi_vec3(b.head.into()),
            tail: to_ffi_vec3(b.tail.into()),
        })
        .collect();

    let ffi_armature = BlenderShimArmatureDesc {
        bones: ffi_bones.as_ptr(),
        bone_count: ffi_bones.len() as i32,
    };


    let blend_path = CString::new(
        blend_path
            .to_str()
            .ok_or_else(|| "blend_path is not valid UTF-8".to_string())?,
    )
        .map_err(|_| "blend_path contains interior NUL byte".to_string())?;

    let mut err_buf = vec![0u8; 1024];

    let result = unsafe {
        blender_shim_write_armature_desc_to_blend(
            &ffi_armature,
            blend_path.as_ptr(),
            err_buf.as_mut_ptr() as *mut c_char,
            err_buf.len() as c_int,
        )
    };

    if result.ok != 0 {
        Ok(())
    } else {
        let msg = unsafe {
            CStr::from_ptr(err_buf.as_ptr() as *const c_char)
                .to_string_lossy()
                .into_owned()
        };
        Err(if msg.is_empty() {
            "failed to write blend file".to_string()
        } else {
            msg
        })
    }

}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlenderShimWriteBlendResult {
    pub ok: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let (major, minor, patch) = crate::blender_version();
        let version = crate::blender_version_string();

        println!("Blender version from C++ shim: {version} ({major}.{minor}.{patch})");

        assert!(major >= 1);
        assert_eq!(version, format!("{major}.{minor}.{patch}"));
    }


    #[test]
    fn normalize_vec3_works() {
        let (out, len) = normalize_vec3([3.0, 4.0, 0.0]);

        approx_eq(len, 5.0, 1e-6);
        approx_eq(out[0], 0.6, 1e-6);
        approx_eq(out[1], 0.8, 1e-6);
        approx_eq(out[2], 0.0, 1e-6);
    }

    #[test]
    fn dot_vec3_works() {
        let dot = dot_vec3([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
        approx_eq(dot, 32.0, 1e-6);
    }

    fn approx_eq(a: f32, b: f32, eps: f32) {
        assert!(
            (a - b).abs() <= eps,
            "expected {a} ~= {b} (eps={eps})"
        );
    }


    #[test]
    fn test_write_blend_file() {

        let armature = make_simple_torso_armature();

        let path = std::env::temp_dir().join("blender_shims_simple_torso_integration.blend");
        write_armature_desc_to_blend(&armature, &path).unwrap();

        println!("written: {}", path.display());

        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 0);
    }
}
