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

#ifdef __cplusplus
}
#endif

#endif //MESH_FIT_BLENDER_SHIM_H
