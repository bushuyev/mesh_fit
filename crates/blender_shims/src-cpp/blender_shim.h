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

/**
 * Writes something like "5.1.0" into `out`, always NUL-terminated if out_size > 0.
 * Returns the number of characters that would have been written, excluding NUL.
 */
int blender_shim_version_string(char *out, int out_size);

#ifdef __cplusplus
}
#endif

#endif //MESH_FIT_BLENDER_SHIM_H
