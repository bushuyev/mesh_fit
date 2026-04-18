//
// Created by bu on 4/18/26.
//

#ifndef BLENDER_SHIM_UTILS_H
#define BLENDER_SHIM_UTILS_H
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

#endif //BLENDER_SHIM_UTILS_H


