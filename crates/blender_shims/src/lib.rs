use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn blender_shim_version_major() -> c_int;
    fn blender_shim_version_minor() -> c_int;
    fn blender_shim_version_patch() -> c_int;
    fn blender_shim_version_string(out: *mut c_char, out_size: c_int) -> c_int;

    fn blender_shim_normalize_vec3(input: *const f32, output: *mut f32) -> f32;
    fn blender_shim_dot_vec3(a: *const f32, b: *const f32) -> f32;

    fn blender_shim_make_bone_from_joints(
        joint_a: BlenderShimVec3,
        joint_b: BlenderShimVec3,
    ) -> BlenderShimBoneFromJointsResult;

    fn blender_shim_debug_print_bone_from_joints(
        label: *const c_char,
        joint_a: BlenderShimVec3,
        joint_b: BlenderShimVec3,
    );

    fn blender_shim_fit_simple_chain(
        joints: *const BlenderShimNamedJoint,
        joint_count: c_int,
    ) -> BlenderShimSimpleChainResult;

    fn blender_shim_debug_print_simple_chain(
        joints: *const BlenderShimNamedJoint,
        joint_count: c_int,
    );

    fn blender_shim_compute_torso_landmarks(
        joints: *const BlenderShimNamedJoint,
        joint_count: c_int,
    ) -> BlenderShimTorsoLandmarksResult;

    fn blender_shim_debug_print_torso_landmarks(
        joints: *const BlenderShimNamedJoint,
        joint_count: c_int,
    );
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

fn from_ffi_vec3(v: BlenderShimVec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

pub fn make_bone_from_joints(joint_a: [f32; 3], joint_b: [f32; 3]) -> BoneFromJoints {
    let result = unsafe { blender_shim_make_bone_from_joints(to_ffi_vec3(joint_a), to_ffi_vec3(joint_b)) };

    BoneFromJoints {
        head: from_ffi_vec3(result.head),
        tail: from_ffi_vec3(result.tail),
        direction_unit: from_ffi_vec3(result.direction_unit),
        length: result.length,
        ok: result.ok != 0,
    }
}

pub fn debug_print_bone_from_joints(label: &str, joint_a: [f32; 3], joint_b: [f32; 3]) {
    let label = CString::new(label).expect("label must not contain interior NUL bytes");
    unsafe {
        blender_shim_debug_print_bone_from_joints(
            label.as_ptr(),
            to_ffi_vec3(joint_a),
            to_ffi_vec3(joint_b),
        );
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

fn to_ffi_named_joint(j: NamedJoint) -> BlenderShimNamedJoint {
    BlenderShimNamedJoint {
        joint_id: j.joint_id as i32,
        position: to_ffi_vec3(j.position),
        confidence: j.confidence,
    }
}

fn from_ffi_bone(r: BlenderShimBoneFromJointsResult) -> BoneFromJoints {
    BoneFromJoints {
        head: from_ffi_vec3(r.head),
        tail: from_ffi_vec3(r.tail),
        direction_unit: from_ffi_vec3(r.direction_unit),
        length: r.length,
        ok: r.ok != 0,
    }
}


pub fn fit_simple_chain(joints: &[NamedJoint]) -> SimpleChain {
    let ffi_joints: Vec<BlenderShimNamedJoint> =
        joints.iter().copied().map(to_ffi_named_joint).collect();

    let result = unsafe {
        blender_shim_fit_simple_chain(ffi_joints.as_ptr(), ffi_joints.len() as c_int)
    };

    SimpleChain {
        pelvis_found: result.pelvis_found != 0,
        spine_found: result.spine_found != 0,
        neck_found: result.neck_found != 0,

        pelvis: from_ffi_vec3(result.pelvis),
        spine: from_ffi_vec3(result.spine),
        neck: from_ffi_vec3(result.neck),

        pelvis_to_spine: from_ffi_bone(result.pelvis_to_spine),
        spine_to_neck: from_ffi_bone(result.spine_to_neck),
    }
}

pub fn debug_print_simple_chain(joints: &[NamedJoint]) {
    let ffi_joints: Vec<BlenderShimNamedJoint> =
        joints.iter().copied().map(to_ffi_named_joint).collect();

    unsafe {
        blender_shim_debug_print_simple_chain(ffi_joints.as_ptr(), ffi_joints.len() as c_int);
    }
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


pub fn compute_torso_landmarks(joints: &[NamedJoint]) -> TorsoLandmarks {
    let ffi_joints: Vec<BlenderShimNamedJoint> =
        joints.iter().copied().map(to_ffi_named_joint).collect();

    let result = unsafe {
        blender_shim_compute_torso_landmarks(ffi_joints.as_ptr(), ffi_joints.len() as c_int)
    };

    TorsoLandmarks {
        pelvis_found: result.pelvis_found != 0,
        neck_found: result.neck_found != 0,
        left_shoulder_found: result.left_shoulder_found != 0,
        right_shoulder_found: result.right_shoulder_found != 0,
        left_hip_found: result.left_hip_found != 0,
        right_hip_found: result.right_hip_found != 0,

        pelvis_center_ok: result.pelvis_center_ok != 0,
        shoulder_center_ok: result.shoulder_center_ok != 0,
        torso_up_ok: result.torso_up_ok != 0,
        shoulder_axis_ok: result.shoulder_axis_ok != 0,
        hip_axis_ok: result.hip_axis_ok != 0,

        pelvis: from_ffi_vec3(result.pelvis),
        neck: from_ffi_vec3(result.neck),
        left_shoulder: from_ffi_vec3(result.left_shoulder),
        right_shoulder: from_ffi_vec3(result.right_shoulder),
        left_hip: from_ffi_vec3(result.left_hip),
        right_hip: from_ffi_vec3(result.right_hip),

        pelvis_center: from_ffi_vec3(result.pelvis_center),
        shoulder_center: from_ffi_vec3(result.shoulder_center),

        torso_up: from_ffi_bone(result.torso_up),
        shoulder_axis: from_ffi_bone(result.shoulder_axis),
        hip_axis: from_ffi_bone(result.hip_axis),
    }
}

pub fn debug_print_torso_landmarks(joints: &[NamedJoint]) {
    let ffi_joints: Vec<BlenderShimNamedJoint> =
        joints.iter().copied().map(to_ffi_named_joint).collect();

    unsafe {
        blender_shim_debug_print_torso_landmarks(ffi_joints.as_ptr(), ffi_joints.len() as c_int);
    }
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
    fn make_bone_from_joints_works() {
        let bone = make_bone_from_joints([1.0, 2.0, 3.0], [1.0, 2.0, 5.0]);

        assert!(bone.ok);
        assert_eq!(bone.head, [1.0, 2.0, 3.0]);
        assert_eq!(bone.tail, [1.0, 2.0, 5.0]);
        approx_eq(bone.length, 2.0, 1e-6);
        approx_eq(bone.direction_unit[0], 0.0, 1e-6);
        approx_eq(bone.direction_unit[1], 0.0, 1e-6);
        approx_eq(bone.direction_unit[2], 1.0, 1e-6);
    }

    #[test]
    fn make_bone_from_joints_detects_degenerate_case() {
        let bone = make_bone_from_joints([1.0, 1.0, 1.0], [1.0, 1.0, 1.0]);

        assert!(!bone.ok);
        approx_eq(bone.length, 0.0, 1e-6);
        assert_eq!(bone.direction_unit, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn print_joint_bridge_results() {
        let pelvis = [0.0_f32, 0.0, 1.0];
        let spine = [0.0_f32, 0.0, 1.5];
        let left_shoulder = [-0.2_f32, 0.0, 1.7];
        let left_elbow = [-0.5_f32, 0.1, 1.55];

        let torso = make_bone_from_joints(pelvis, spine);
        println!("torso bone: {torso:?}");

        let upper_arm = make_bone_from_joints(left_shoulder, left_elbow);
        println!("upper_arm bone: {upper_arm:?}");

        debug_print_bone_from_joints("torso", pelvis, spine);
        debug_print_bone_from_joints("upper_arm.L", left_shoulder, left_elbow);

        assert!(torso.ok);
        assert!(upper_arm.ok);
    }


    #[test]
    fn fit_simple_chain_works() {
        let joints = [
            NamedJoint {
                joint_id: JointId::Pelvis,
                position: [0.0, 0.0, 1.0],
                confidence: 1.0,
            },
            NamedJoint {
                joint_id: JointId::Spine,
                position: [0.0, 0.0, 1.4],
                confidence: 1.0,
            },
            NamedJoint {
                joint_id: JointId::Neck,
                position: [0.0, 0.0, 1.7],
                confidence: 1.0,
            },
        ];

        let chain = fit_simple_chain(&joints);

        assert!(chain.pelvis_found);
        assert!(chain.spine_found);
        assert!(chain.neck_found);

        assert!(chain.pelvis_to_spine.ok);
        assert!(chain.spine_to_neck.ok);

        approx_eq(chain.pelvis_to_spine.length, 0.4, 1e-6);
        approx_eq(chain.spine_to_neck.length, 0.3, 1e-6);
    }


    #[test]
    fn print_simple_chain() {
        let joints = [
            NamedJoint {
                joint_id: JointId::Pelvis,
                position: [0.0, 0.0, 1.0],
                confidence: 0.99,
            },
            NamedJoint {
                joint_id: JointId::Spine,
                position: [0.0, 0.0, 1.35],
                confidence: 0.98,
            },
            NamedJoint {
                joint_id: JointId::Neck,
                position: [0.0, 0.0, 1.65],
                confidence: 0.97,
            },
        ];

        let chain = fit_simple_chain(&joints);
        println!("chain: {chain:?}");

        debug_print_simple_chain(&joints);

        assert!(chain.pelvis_to_spine.ok);
        assert!(chain.spine_to_neck.ok);
    }


    #[test]
    fn compute_torso_landmarks_works() {
        let joints = [
            NamedJoint {
                joint_id: JointId::Pelvis,
                position: [0.0, 0.0, 1.0],
                confidence: 1.0,
            },
            NamedJoint {
                joint_id: JointId::Neck,
                position: [0.0, 0.0, 1.6],
                confidence: 1.0,
            },
            NamedJoint {
                joint_id: JointId::LeftShoulder,
                position: [-0.2, 0.0, 1.5],
                confidence: 1.0,
            },
            NamedJoint {
                joint_id: JointId::RightShoulder,
                position: [0.2, 0.0, 1.5],
                confidence: 1.0,
            },
            NamedJoint {
                joint_id: JointId::LeftHip,
                position: [-0.15, 0.0, 1.0],
                confidence: 1.0,
            },
            NamedJoint {
                joint_id: JointId::RightHip,
                position: [0.15, 0.0, 1.0],
                confidence: 1.0,
            },
        ];

        let torso = compute_torso_landmarks(&joints);

        assert!(torso.pelvis_center_ok);
        assert!(torso.shoulder_center_ok);
        assert!(torso.torso_up_ok);
        assert!(torso.shoulder_axis_ok);
        assert!(torso.hip_axis_ok);

        approx_eq(torso.pelvis_center[0], 0.0, 1e-6);
        approx_eq(torso.pelvis_center[1], 0.0, 1e-6);
        approx_eq(torso.pelvis_center[2], 1.0, 1e-6);

        approx_eq(torso.shoulder_center[0], 0.0, 1e-6);
        approx_eq(torso.shoulder_center[2], 1.5, 1e-6);

        approx_eq(torso.torso_up.length, 0.6, 1e-6);
        approx_eq(torso.shoulder_axis.length, 0.4, 1e-6);
        approx_eq(torso.hip_axis.length, 0.3, 1e-6);
    }

    #[test]
    fn print_torso_landmarks() {
        let joints = [
            NamedJoint {
                joint_id: JointId::Pelvis,
                position: [0.0, 0.0, 1.0],
                confidence: 0.99,
            },
            NamedJoint {
                joint_id: JointId::Neck,
                position: [0.0, 0.0, 1.62],
                confidence: 0.97,
            },
            NamedJoint {
                joint_id: JointId::LeftShoulder,
                position: [-0.23, 0.02, 1.50],
                confidence: 0.98,
            },
            NamedJoint {
                joint_id: JointId::RightShoulder,
                position: [0.21, -0.01, 1.49],
                confidence: 0.98,
            },
            NamedJoint {
                joint_id: JointId::LeftHip,
                position: [-0.14, 0.01, 1.01],
                confidence: 0.96,
            },
            NamedJoint {
                joint_id: JointId::RightHip,
                position: [0.16, -0.02, 0.99],
                confidence: 0.96,
            },
        ];

        let torso = compute_torso_landmarks(&joints);
        println!("torso: {torso:?}");

        debug_print_torso_landmarks(&joints);

        assert!(torso.torso_up_ok);
        assert!(torso.shoulder_axis_ok);
        assert!(torso.hip_axis_ok);
    }
}
