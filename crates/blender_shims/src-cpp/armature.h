//
// Created by bu on 4/18/26.
//

#ifndef BLENDER_SHIM_ARMATURE_H
#define BLENDER_SHIM_ARMATURE_H
extern "C" {
typedef struct BlenderShimVec3 {
    float x;
    float y;
    float z;
} BlenderShimVec3;

typedef struct BlenderShimBoneFromJointsResult {
    BlenderShimVec3 head;
    BlenderShimVec3 tail;
    BlenderShimVec3 direction_unit;
    float length;
    int ok;
} BlenderShimBoneFromJointsResult;


typedef struct BlenderShimNamedJoint {
    int joint_id;
    BlenderShimVec3 position;
    float confidence;
} BlenderShimNamedJoint;


typedef struct BlenderShimTorsoLandmarksResult {
    int pelvis_found;
    int neck_found;
    int left_shoulder_found;
    int right_shoulder_found;
    int left_hip_found;
    int right_hip_found;

    int pelvis_center_ok;
    int shoulder_center_ok;
    int torso_up_ok;
    int shoulder_axis_ok;
    int hip_axis_ok;

    BlenderShimVec3 pelvis;
    BlenderShimVec3 neck;
    BlenderShimVec3 left_shoulder;
    BlenderShimVec3 right_shoulder;
    BlenderShimVec3 left_hip;
    BlenderShimVec3 right_hip;

    BlenderShimVec3 pelvis_center;
    BlenderShimVec3 shoulder_center;

    BlenderShimBoneFromJointsResult torso_up;
    BlenderShimBoneFromJointsResult shoulder_axis;
    BlenderShimBoneFromJointsResult hip_axis;
} BlenderShimTorsoLandmarksResult;

BlenderShimTorsoLandmarksResult blender_shim_compute_torso_landmarks(
    const BlenderShimNamedJoint *joints,
    int joint_count);


typedef struct BlenderShimBasis3 {
    BlenderShimVec3 x_axis;
    BlenderShimVec3 y_axis;
    BlenderShimVec3 z_axis;
    int ok;
} BlenderShimBasis3;

typedef struct BlenderShimTorsoFrameResult {
    BlenderShimTorsoLandmarksResult landmarks;
    BlenderShimVec3 origin;
    BlenderShimBasis3 basis;
} BlenderShimTorsoFrameResult;


typedef struct BlenderShimBoneDesc {
    const char *name;
    int parent_index; /* -1 if root */
    BlenderShimVec3 head;
    BlenderShimVec3 tail;
} BlenderShimBoneDesc;

typedef struct BlenderShimArmatureDesc {
    const BlenderShimBoneDesc *bones;
    int bone_count;
} BlenderShimArmatureDesc;

typedef struct BlenderShimArmatureValidationResult {
    int ok;
    int has_invalid_parent;
    int has_degenerate_bone;
    int first_invalid_bone_index;
} BlenderShimArmatureValidationResult;


typedef struct BlenderShimWriteBlendResult {
    int ok;
} BlenderShimWriteBlendResult;

BlenderShimWriteBlendResult blender_shim_write_armature_desc_to_blend(
    const BlenderShimArmatureDesc *armature,
    const char *blend_path,
    char *error_out,
    int error_out_size);
}
#endif //BLENDER_SHIM_ARMATURE_H
