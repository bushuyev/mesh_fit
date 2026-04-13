#include "blender_shim.h"

#include <cstdio>

#include "BKE_blender_version.h"
#include "BLI_math_vector.h"

using namespace blender;

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
    return normalize_v3(out);
}

float blender_shim_dot_vec3(const float a[3], const float b[3]) {
    return dot_v3v3(a, b);
}


static void vec3_to_array(const BlenderShimVec3 &v, float out[3]) {
    out[0] = v.x;
    out[1] = v.y;
    out[2] = v.z;
}

static BlenderShimVec3 array_to_vec3(const float v[3]) {
    BlenderShimVec3 out{};
    out.x = v[0];
    out.y = v[1];
    out.z = v[2];
    return out;
}

BlenderShimBoneFromJointsResult blender_shim_make_bone_from_joints(
    BlenderShimVec3 joint_a,
    BlenderShimVec3 joint_b)
{
    BlenderShimBoneFromJointsResult result{};
    result.head = joint_a;
    result.tail = joint_b;
    result.direction_unit = BlenderShimVec3{0.0f, 0.0f, 0.0f};
    result.length = 0.0f;
    result.ok = 0;

    float a[3];
    float b[3];
    float delta[3];

    vec3_to_array(joint_a, a);
    vec3_to_array(joint_b, b);

    sub_v3_v3v3(delta, b, a);
    const float len = normalize_v3(delta);

    result.length = len;
    if (len > 1.0e-8f) {
        result.direction_unit = array_to_vec3(delta);
        result.ok = 1;
    }

    return result;
}

void blender_shim_debug_print_bone_from_joints(
    const char *label,
    BlenderShimVec3 joint_a,
    BlenderShimVec3 joint_b)
{
    const BlenderShimBoneFromJointsResult r =
        blender_shim_make_bone_from_joints(joint_a, joint_b);

    const char *safe_label = (label != nullptr) ? label : "bone";

    std::printf(
        "[blender_shim] %s: "
        "head=(%.6f, %.6f, %.6f) "
        "tail=(%.6f, %.6f, %.6f) "
        "dir=(%.6f, %.6f, %.6f) "
        "len=%.6f ok=%d\n",
        safe_label,
        r.head.x, r.head.y, r.head.z,
        r.tail.x, r.tail.y, r.tail.z,
        r.direction_unit.x, r.direction_unit.y, r.direction_unit.z,
        r.length,
        r.ok);
    std::fflush(stdout);
}