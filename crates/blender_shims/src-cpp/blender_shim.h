//
// Created by bu on 4/13/26.
//

#ifndef MESH_FIT_BLENDER_SHIM_H
#define MESH_FIT_BLENDER_SHIM_H

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

#include "utils.h"
int blender_shim_version_major();

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

/**
 * Build a simple "bone" from two joint positions:
 * - head = joint_a
 * - tail = joint_b
 * - direction_unit = normalized (joint_b - joint_a)
 * - length = |joint_b - joint_a|
 *
 * ok = 0 if the two joints are too close / degenerate.
 */
// BlenderShimBoneFromJointsResult blender_shim_make_bone_from_joints( BlenderShimVec3 joint_a, BlenderShimVec3 joint_b);

/**
 * Debug helper that prints a bone computed from two joints.
 */
// void blender_shim_debug_print_bone_from_joints( const char *label, BlenderShimVec3 joint_a, BlenderShimVec3 joint_b);



typedef enum BlenderShimJointId {
    BLENDER_SHIM_JOINT_PELVIS = 0,
    BLENDER_SHIM_JOINT_SPINE = 1,
    BLENDER_SHIM_JOINT_NECK = 2,

    BLENDER_SHIM_JOINT_LEFT_SHOULDER = 3,
    BLENDER_SHIM_JOINT_RIGHT_SHOULDER = 4,
    BLENDER_SHIM_JOINT_LEFT_HIP = 5,
    BLENDER_SHIM_JOINT_RIGHT_HIP = 6,
} BlenderShimJointId;

typedef struct BlenderShimNamedJoint {
    int joint_id;
    BlenderShimVec3 position;
    float confidence;
} BlenderShimNamedJoint;

typedef struct BlenderShimSimpleChainResult {
    int pelvis_found;
    int spine_found;
    int neck_found;

    BlenderShimVec3 pelvis;
    BlenderShimVec3 spine;
    BlenderShimVec3 neck;

    BlenderShimBoneFromJointsResult pelvis_to_spine;
    BlenderShimBoneFromJointsResult spine_to_neck;
} BlenderShimSimpleChainResult;

/**
 * Finds pelvis/spine/neck in the provided joint array and builds a simple chain.
 * Missing joints are reported via *_found = 0.
 */
// BlenderShimSimpleChainResult blender_shim_fit_simple_chain( const BlenderShimNamedJoint *joints, int joint_count);

/**
 * Debug helper that prints the fitted simple chain.
 */
// void blender_shim_debug_print_simple_chain( const BlenderShimNamedJoint *joints, int joint_count);


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

void blender_shim_debug_print_torso_landmarks(
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

BlenderShimTorsoFrameResult blender_shim_compute_torso_frame(
    const BlenderShimNamedJoint *joints,
    int joint_count);

void blender_shim_debug_print_torso_frame(
    const BlenderShimNamedJoint *joints,
    int joint_count);



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

// BlenderShimArmatureValidationResult blender_shim_validate_armature_desc(
    // const BlenderShimArmatureDesc *armature);

// void blender_shim_debug_print_armature_desc(
//     const BlenderShimArmatureDesc *armature);

typedef struct BlenderShimWriteBlendResult {
    int ok;
} BlenderShimWriteBlendResult;

BlenderShimWriteBlendResult blender_shim_write_armature_desc_to_blend(
    const BlenderShimArmatureDesc *armature,
    const char *blend_path,
    char *error_out,
    int error_out_size);

#ifdef __cplusplus
}
#endif

#endif //MESH_FIT_BLENDER_SHIM_H
