#include "blender_shim.h"

#include <cstdio>

#include "BKE_blender_version.h"
#include "BLI_math_vector.h"

int blender_shim_version_major() {
    return BLENDER_VERSION / 100;
}

int blender_shim_version_minor() {
    return BLENDER_VERSION % 100;
}

int blender_shim_version_patch() {
    return BLENDER_VERSION_PATCH;
}

int blender_shim_version_string(char *out, int out_size) {
    const int major = blender_shim_version_major();
    const int minor = blender_shim_version_minor();
    const int patch = blender_shim_version_patch();

    if (out == nullptr || out_size <= 0) {
        return std::snprintf(nullptr, 0, "%d.%d.%d", major, minor, patch);
    }

    return std::snprintf(out, static_cast<std::size_t>(out_size), "%d.%d.%d", major, minor, patch);
}

float blender_shim_normalize_vec3(const float in[3], float out[3]) {
    out[0] = in[0];
    out[1] = in[1];
    out[2] = in[2];
    return blender::normalize_v3(out);
}

float blender_shim_dot_vec3(const float a[3], const float b[3]) {
    return blender::dot_v3v3(a, b);
}