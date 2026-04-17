use meshfit_shared::armature::{ArmatureDesc, BoneDesc};
use meshfit_shared::armature::Keypoint3D;

pub fn make_rtmw3d_armature(points: &[Keypoint3D; 133]) -> ArmatureDesc {
    #[derive(Clone, Copy)]
    enum JointRef {
        K(usize),
        PelvisCenter,
        ShoulderCenter,
        Neck,
    }

    #[derive(Clone, Copy)]
    struct BoneSpec {
        name: &'static str,
        head: JointRef,
        tail: JointRef,
    }

    fn midpoint(a: Keypoint3D, b: Keypoint3D) -> Keypoint3D {
        Keypoint3D {
            x: (a.x + b.x) * 0.5,
            y: (a.y + b.y) * 0.5,
            z_rel: (a.z_rel + b.z_rel) * 0.5,
        }
    }

    let pelvis_center = midpoint(points[11], points[12]);     // left_hip, right_hip
    let shoulder_center = midpoint(points[5], points[6]);     // left_shoulder, right_shoulder
    let neck = midpoint(shoulder_center, points[0]);          // between shoulders and nose

    let joint_pos = |j: JointRef| -> Keypoint3D {
        match j {
            JointRef::K(i) => points[i],
            JointRef::PelvisCenter => pelvis_center,
            JointRef::ShoulderCenter => shoulder_center,
            JointRef::Neck => neck,
        }
    };

    let mut bones = Vec::<BoneDesc>::new();

    let add_bone = |bones: &mut Vec<BoneDesc>,
                    name: &str,
                    parent_index: i32,
                    head: Keypoint3D,
                    tail: Keypoint3D| {
        bones.push(BoneDesc {
            name: name.to_string(),
            parent_index,
            head,
            tail,
        });
        (bones.len() - 1) as i32
    };

    let add_chain = |bones: &mut Vec<BoneDesc>,
                     prefix: &str,
                     parent_index: i32,
                     chain: &[JointRef]| {
        let mut parent = parent_index;
        for i in 0..chain.len() - 1 {
            let name = format!("{prefix}_{i:02}");
            parent = add_bone(
                bones,
                &name,
                parent,
                joint_pos(chain[i]),
                joint_pos(chain[i + 1]),
            );
        }
        parent
    };

    // torso
    let pelvis_i = add_bone(
        &mut bones,
        "pelvis",
        -1,
        joint_pos(JointRef::PelvisCenter),
        joint_pos(JointRef::ShoulderCenter),
    );

    let spine_i = add_bone(
        &mut bones,
        "spine",
        pelvis_i,
        joint_pos(JointRef::ShoulderCenter),
        joint_pos(JointRef::Neck),
    );

    // arms
    let clav_l_i = add_bone(
        &mut bones,
        "clavicle.L",
        spine_i,
        joint_pos(JointRef::Neck),
        joint_pos(JointRef::K(5)),
    );
    let upper_arm_l_i = add_bone(
        &mut bones,
        "upper_arm.L",
        clav_l_i,
        joint_pos(JointRef::K(5)),
        joint_pos(JointRef::K(7)),
    );
    let forearm_l_i = add_bone(
        &mut bones,
        "forearm.L",
        upper_arm_l_i,
        joint_pos(JointRef::K(7)),
        joint_pos(JointRef::K(9)),
    );

    let clav_r_i = add_bone(
        &mut bones,
        "clavicle.R",
        spine_i,
        joint_pos(JointRef::Neck),
        joint_pos(JointRef::K(6)),
    );
    let upper_arm_r_i = add_bone(
        &mut bones,
        "upper_arm.R",
        clav_r_i,
        joint_pos(JointRef::K(6)),
        joint_pos(JointRef::K(8)),
    );
    let forearm_r_i = add_bone(
        &mut bones,
        "forearm.R",
        upper_arm_r_i,
        joint_pos(JointRef::K(8)),
        joint_pos(JointRef::K(10)),
    );

    // legs
    let hip_l_i = add_bone(
        &mut bones,
        "hip.L",
        pelvis_i,
        joint_pos(JointRef::PelvisCenter),
        joint_pos(JointRef::K(11)),
    );
    let thigh_l_i = add_bone(
        &mut bones,
        "thigh.L",
        hip_l_i,
        joint_pos(JointRef::K(11)),
        joint_pos(JointRef::K(13)),
    );
    let shin_l_i = add_bone(
        &mut bones,
        "shin.L",
        thigh_l_i,
        joint_pos(JointRef::K(13)),
        joint_pos(JointRef::K(15)),
    );

    let hip_r_i = add_bone(
        &mut bones,
        "hip.R",
        pelvis_i,
        joint_pos(JointRef::PelvisCenter),
        joint_pos(JointRef::K(12)),
    );
    let thigh_r_i = add_bone(
        &mut bones,
        "thigh.R",
        hip_r_i,
        joint_pos(JointRef::K(12)),
        joint_pos(JointRef::K(14)),
    );
    let shin_r_i = add_bone(
        &mut bones,
        "shin.R",
        thigh_r_i,
        joint_pos(JointRef::K(14)),
        joint_pos(JointRef::K(16)),
    );

    // feet
    let _ = add_bone(
        &mut bones,
        "foot_big_toe.L",
        shin_l_i,
        joint_pos(JointRef::K(15)),
        joint_pos(JointRef::K(17)),
    );
    let _ = add_bone(
        &mut bones,
        "foot_small_toe.L",
        shin_l_i,
        joint_pos(JointRef::K(15)),
        joint_pos(JointRef::K(18)),
    );
    let _ = add_bone(
        &mut bones,
        "foot_heel.L",
        shin_l_i,
        joint_pos(JointRef::K(15)),
        joint_pos(JointRef::K(19)),
    );

    let _ = add_bone(
        &mut bones,
        "foot_big_toe.R",
        shin_r_i,
        joint_pos(JointRef::K(16)),
        joint_pos(JointRef::K(20)),
    );
    let _ = add_bone(
        &mut bones,
        "foot_small_toe.R",
        shin_r_i,
        joint_pos(JointRef::K(16)),
        joint_pos(JointRef::K(21)),
    );
    let _ = add_bone(
        &mut bones,
        "foot_heel.R",
        shin_r_i,
        joint_pos(JointRef::K(16)),
        joint_pos(JointRef::K(22)),
    );

    // coarse head
    let head_i = add_bone(
        &mut bones,
        "head",
        spine_i,
        joint_pos(JointRef::Neck),
        joint_pos(JointRef::K(0)),
    );

    let eye_l_i = add_bone(
        &mut bones,
        "eye.L",
        head_i,
        joint_pos(JointRef::K(0)),
        joint_pos(JointRef::K(1)),
    );
    let eye_r_i = add_bone(
        &mut bones,
        "eye.R",
        head_i,
        joint_pos(JointRef::K(0)),
        joint_pos(JointRef::K(2)),
    );

    let _ = add_bone(
        &mut bones,
        "ear.L",
        eye_l_i,
        joint_pos(JointRef::K(1)),
        joint_pos(JointRef::K(3)),
    );
    let _ = add_bone(
        &mut bones,
        "ear.R",
        eye_r_i,
        joint_pos(JointRef::K(2)),
        joint_pos(JointRef::K(4)),
    );

    // face: 68 points = 23..90
    // jaw 23..39
    let _ = add_chain(
        &mut bones,
        "jaw",
        head_i,
        &[
            JointRef::K(23), JointRef::K(24), JointRef::K(25), JointRef::K(26), JointRef::K(27),
            JointRef::K(28), JointRef::K(29), JointRef::K(30), JointRef::K(31), JointRef::K(32),
            JointRef::K(33), JointRef::K(34), JointRef::K(35), JointRef::K(36), JointRef::K(37),
            JointRef::K(38), JointRef::K(39),
        ],
    );

    // eyebrows
    let _ = add_chain(
        &mut bones,
        "brow.R",
        head_i,
        &[
            JointRef::K(40), JointRef::K(41), JointRef::K(42), JointRef::K(43), JointRef::K(44),
        ],
    );
    let _ = add_chain(
        &mut bones,
        "brow.L",
        head_i,
        &[
            JointRef::K(45), JointRef::K(46), JointRef::K(47), JointRef::K(48), JointRef::K(49),
        ],
    );

    // nose
    let nose_bridge_i = add_chain(
        &mut bones,
        "nose_bridge",
        head_i,
        &[
            JointRef::K(50), JointRef::K(51), JointRef::K(52), JointRef::K(53),
        ],
    );

    let _ = add_chain(
        &mut bones,
        "nose_lower",
        nose_bridge_i,
        &[
            JointRef::K(54), JointRef::K(55), JointRef::K(56), JointRef::K(57), JointRef::K(58),
        ],
    );

    // eye loops as open chains
    let _ = add_chain(
        &mut bones,
        "eye_loop.R",
        head_i,
        &[
            JointRef::K(59), JointRef::K(60), JointRef::K(61),
            JointRef::K(62), JointRef::K(63), JointRef::K(64),
        ],
    );
    let _ = add_chain(
        &mut bones,
        "eye_loop.L",
        head_i,
        &[
            JointRef::K(65), JointRef::K(66), JointRef::K(67),
            JointRef::K(68), JointRef::K(69), JointRef::K(70),
        ],
    );

    // mouth
    let mouth_outer_i = add_chain(
        &mut bones,
        "lip_outer",
        head_i,
        &[
            JointRef::K(71), JointRef::K(72), JointRef::K(73), JointRef::K(74),
            JointRef::K(75), JointRef::K(76), JointRef::K(77), JointRef::K(78),
            JointRef::K(79), JointRef::K(80), JointRef::K(81), JointRef::K(82),
        ],
    );

    let _ = add_chain(
        &mut bones,
        "lip_inner",
        mouth_outer_i,
        &[
            JointRef::K(83), JointRef::K(84), JointRef::K(85), JointRef::K(86),
            JointRef::K(87), JointRef::K(88), JointRef::K(89), JointRef::K(90),
        ],
    );

    // left hand: 91..111
    let hand_l_i = add_bone(
        &mut bones,
        "hand.L",
        forearm_l_i,
        joint_pos(JointRef::K(9)),
        joint_pos(JointRef::K(91)),
    );

    let _ = add_chain(
        &mut bones,
        "thumb.L",
        hand_l_i,
        &[JointRef::K(91), JointRef::K(92), JointRef::K(93), JointRef::K(94), JointRef::K(95)],
    );
    let _ = add_chain(
        &mut bones,
        "index.L",
        hand_l_i,
        &[JointRef::K(91), JointRef::K(96), JointRef::K(97), JointRef::K(98), JointRef::K(99)],
    );
    let _ = add_chain(
        &mut bones,
        "middle.L",
        hand_l_i,
        &[JointRef::K(91), JointRef::K(100), JointRef::K(101), JointRef::K(102), JointRef::K(103)],
    );
    let _ = add_chain(
        &mut bones,
        "ring.L",
        hand_l_i,
        &[JointRef::K(91), JointRef::K(104), JointRef::K(105), JointRef::K(106), JointRef::K(107)],
    );
    let _ = add_chain(
        &mut bones,
        "pinky.L",
        hand_l_i,
        &[JointRef::K(91), JointRef::K(108), JointRef::K(109), JointRef::K(110), JointRef::K(111)],
    );

    // right hand: 112..132
    let hand_r_i = add_bone(
        &mut bones,
        "hand.R",
        forearm_r_i,
        joint_pos(JointRef::K(10)),
        joint_pos(JointRef::K(112)),
    );

    let _ = add_chain(
        &mut bones,
        "thumb.R",
        hand_r_i,
        &[JointRef::K(112), JointRef::K(113), JointRef::K(114), JointRef::K(115), JointRef::K(116)],
    );
    let _ = add_chain(
        &mut bones,
        "index.R",
        hand_r_i,
        &[JointRef::K(112), JointRef::K(117), JointRef::K(118), JointRef::K(119), JointRef::K(120)],
    );
    let _ = add_chain(
        &mut bones,
        "middle.R",
        hand_r_i,
        &[JointRef::K(112), JointRef::K(121), JointRef::K(122), JointRef::K(123), JointRef::K(124)],
    );
    let _ = add_chain(
        &mut bones,
        "ring.R",
        hand_r_i,
        &[JointRef::K(112), JointRef::K(125), JointRef::K(126), JointRef::K(127), JointRef::K(128)],
    );
    let _ = add_chain(
        &mut bones,
        "pinky.R",
        hand_r_i,
        &[JointRef::K(112), JointRef::K(129), JointRef::K(130), JointRef::K(131), JointRef::K(132)],
    );

    ArmatureDesc { bones }
}