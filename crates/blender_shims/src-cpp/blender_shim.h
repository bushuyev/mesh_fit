//
// Created by bu on 4/13/26.
//

#ifndef MESH_FIT_BLENDER_SHIM_H
#define MESH_FIT_BLENDER_SHIM_H

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int blender_shim_version_major();
int blender_shim_version_minor();
int blender_shim_version_patch();
int blender_shim_version_string(char *out, int out_size);

/**
 * Normalizes a 3D vector using Blender math utilities.
 *
 * Returns the original vector length before normalization.
 * If the vector is zero-length, Blender leaves it as-is.
 */
float blender_shim_normalize_vec3(const float in[3], float out[3]);

/**
 * Computes dot product of two 3D vectors using Blender math utilities.
 */
float blender_shim_dot_vec3(const float a[3], const float b[3]);


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
BlenderShimBoneFromJointsResult blender_shim_make_bone_from_joints(
    BlenderShimVec3 joint_a,
    BlenderShimVec3 joint_b);

/**
 * Debug helper that prints a bone computed from two joints.
 */
void blender_shim_debug_print_bone_from_joints(
    const char *label,
    BlenderShimVec3 joint_a,
    BlenderShimVec3 joint_b);

#ifdef __cplusplus
}
#endif

#endif //MESH_FIT_BLENDER_SHIM_H
