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


static int find_joint(
    const BlenderShimNamedJoint *joints,
    int joint_count,
    int joint_id,
    BlenderShimVec3 *out_position)
{
    if (joints == nullptr || joint_count <= 0 || out_position == nullptr) {
        return 0;
    }

    for (int i = 0; i < joint_count; ++i) {
        if (joints[i].joint_id == joint_id) {
            *out_position = joints[i].position;
            return 1;
        }
    }
    return 0;
}

BlenderShimSimpleChainResult blender_shim_fit_simple_chain(
    const BlenderShimNamedJoint *joints,
    int joint_count)
{
    BlenderShimSimpleChainResult result{};
    result.pelvis_found = find_joint(
        joints, joint_count, BLENDER_SHIM_JOINT_PELVIS, &result.pelvis);
    result.spine_found = find_joint(
        joints, joint_count, BLENDER_SHIM_JOINT_SPINE, &result.spine);
    result.neck_found = find_joint(
        joints, joint_count, BLENDER_SHIM_JOINT_NECK, &result.neck);

    if (result.pelvis_found && result.spine_found) {
        result.pelvis_to_spine =
            blender_shim_make_bone_from_joints(result.pelvis, result.spine);
    }

    if (result.spine_found && result.neck_found) {
        result.spine_to_neck =
            blender_shim_make_bone_from_joints(result.spine, result.neck);
    }

    return result;
}

void blender_shim_debug_print_simple_chain(
    const BlenderShimNamedJoint *joints,
    int joint_count)
{
    const BlenderShimSimpleChainResult r =
        blender_shim_fit_simple_chain(joints, joint_count);

    std::printf(
        "[blender_shim] simple_chain: "
        "pelvis_found=%d spine_found=%d neck_found=%d\n",
        r.pelvis_found, r.spine_found, r.neck_found);

    if (r.pelvis_found) {
        std::printf(
            "  pelvis=(%.6f, %.6f, %.6f)\n",
            r.pelvis.x, r.pelvis.y, r.pelvis.z);
    }
    if (r.spine_found) {
        std::printf(
            "  spine =(%.6f, %.6f, %.6f)\n",
            r.spine.x, r.spine.y, r.spine.z);
    }
    if (r.neck_found) {
        std::printf(
            "  neck  =(%.6f, %.6f, %.6f)\n",
            r.neck.x, r.neck.y, r.neck.z);
    }

    if (r.pelvis_to_spine.ok) {
        std::printf(
            "  pelvis->spine len=%.6f dir=(%.6f, %.6f, %.6f)\n",
            r.pelvis_to_spine.length,
            r.pelvis_to_spine.direction_unit.x,
            r.pelvis_to_spine.direction_unit.y,
            r.pelvis_to_spine.direction_unit.z);
    }

    if (r.spine_to_neck.ok) {
        std::printf(
            "  spine->neck   len=%.6f dir=(%.6f, %.6f, %.6f)\n",
            r.spine_to_neck.length,
            r.spine_to_neck.direction_unit.x,
            r.spine_to_neck.direction_unit.y,
            r.spine_to_neck.direction_unit.z);
    }

    std::fflush(stdout);
}


static BlenderShimVec3 midpoint_vec3(BlenderShimVec3 a, BlenderShimVec3 b)
{
    BlenderShimVec3 out{};
    out.x = 0.5f * (a.x + b.x);
    out.y = 0.5f * (a.y + b.y);
    out.z = 0.5f * (a.z + b.z);
    return out;
}

BlenderShimTorsoLandmarksResult blender_shim_compute_torso_landmarks(
    const BlenderShimNamedJoint *joints,
    int joint_count)
{
    BlenderShimTorsoLandmarksResult result{};

    result.pelvis_found =
        find_joint(joints, joint_count, BLENDER_SHIM_JOINT_PELVIS, &result.pelvis);
    result.neck_found =
        find_joint(joints, joint_count, BLENDER_SHIM_JOINT_NECK, &result.neck);
    result.left_shoulder_found =
        find_joint(joints, joint_count, BLENDER_SHIM_JOINT_LEFT_SHOULDER, &result.left_shoulder);
    result.right_shoulder_found =
        find_joint(joints, joint_count, BLENDER_SHIM_JOINT_RIGHT_SHOULDER, &result.right_shoulder);
    result.left_hip_found =
        find_joint(joints, joint_count, BLENDER_SHIM_JOINT_LEFT_HIP, &result.left_hip);
    result.right_hip_found =
        find_joint(joints, joint_count, BLENDER_SHIM_JOINT_RIGHT_HIP, &result.right_hip);

    if (result.left_hip_found && result.right_hip_found) {
        result.pelvis_center = midpoint_vec3(result.left_hip, result.right_hip);
        result.pelvis_center_ok = 1;
        result.hip_axis =
            blender_shim_make_bone_from_joints(result.left_hip, result.right_hip);
        result.hip_axis_ok = result.hip_axis.ok;
    }

    if (result.left_shoulder_found && result.right_shoulder_found) {
        result.shoulder_center = midpoint_vec3(result.left_shoulder, result.right_shoulder);
        result.shoulder_center_ok = 1;
        result.shoulder_axis =
            blender_shim_make_bone_from_joints(result.left_shoulder, result.right_shoulder);
        result.shoulder_axis_ok = result.shoulder_axis.ok;
    }

    if (result.pelvis_center_ok && result.neck_found) {
        result.torso_up =
            blender_shim_make_bone_from_joints(result.pelvis_center, result.neck);
        result.torso_up_ok = result.torso_up.ok;
    }

    return result;
}

void blender_shim_debug_print_torso_landmarks(
    const BlenderShimNamedJoint *joints,
    int joint_count)
{
    const BlenderShimTorsoLandmarksResult r =
        blender_shim_compute_torso_landmarks(joints, joint_count);

    std::printf(
        "[blender_shim] torso_landmarks: "
        "pelvis=%d neck=%d lsho=%d rsho=%d lhip=%d rhip=%d\n",
        r.pelvis_found,
        r.neck_found,
        r.left_shoulder_found,
        r.right_shoulder_found,
        r.left_hip_found,
        r.right_hip_found);

    if (r.pelvis_center_ok) {
        std::printf(
            "  pelvis_center   =(%.6f, %.6f, %.6f)\n",
            r.pelvis_center.x, r.pelvis_center.y, r.pelvis_center.z);
    }
    if (r.shoulder_center_ok) {
        std::printf(
            "  shoulder_center =(%.6f, %.6f, %.6f)\n",
            r.shoulder_center.x, r.shoulder_center.y, r.shoulder_center.z);
    }
    if (r.torso_up_ok) {
        std::printf(
            "  torso_up     len=%.6f dir=(%.6f, %.6f, %.6f)\n",
            r.torso_up.length,
            r.torso_up.direction_unit.x,
            r.torso_up.direction_unit.y,
            r.torso_up.direction_unit.z);
    }
    if (r.shoulder_axis_ok) {
        std::printf(
            "  shoulder_axis len=%.6f dir=(%.6f, %.6f, %.6f)\n",
            r.shoulder_axis.length,
            r.shoulder_axis.direction_unit.x,
            r.shoulder_axis.direction_unit.y,
            r.shoulder_axis.direction_unit.z);
    }
    if (r.hip_axis_ok) {
        std::printf(
            "  hip_axis      len=%.6f dir=(%.6f, %.6f, %.6f)\n",
            r.hip_axis.length,
            r.hip_axis.direction_unit.x,
            r.hip_axis.direction_unit.y,
            r.hip_axis.direction_unit.z);
    }

    std::fflush(stdout);
}




static BlenderShimVec3 make_vec3(float x, float y, float z)
{
    BlenderShimVec3 out{};
    out.x = x;
    out.y = y;
    out.z = z;
    return out;
}

static void copy_vec3(BlenderShimVec3 v, float out[3])
{
    out[0] = v.x;
    out[1] = v.y;
    out[2] = v.z;
}

static BlenderShimVec3 normalized_vec3_or_zero(BlenderShimVec3 v, int *ok)
{
    float a[3];
    copy_vec3(v, a);
    const float len = normalize_v3(a);
    if (len > 1.0e-8f) {
        if (ok != nullptr) {
            *ok = 1;
        }
        return array_to_vec3(a);
    }
    if (ok != nullptr) {
        *ok = 0;
    }
    return make_vec3(0.0f, 0.0f, 0.0f);
}

static BlenderShimVec3 cross_vec3(BlenderShimVec3 a, BlenderShimVec3 b)
{
    float aa[3];
    float bb[3];
    float out[3];
    copy_vec3(a, aa);
    copy_vec3(b, bb);
    cross_v3_v3v3(out, aa, bb);
    return array_to_vec3(out);
}

BlenderShimTorsoFrameResult blender_shim_compute_torso_frame(
    const BlenderShimNamedJoint *joints,
    int joint_count)
{
    BlenderShimTorsoFrameResult result{};
    result.landmarks = blender_shim_compute_torso_landmarks(joints, joint_count);

    if (!result.landmarks.pelvis_center_ok) {
        return result;
    }

    result.origin = result.landmarks.pelvis_center;

    if (!result.landmarks.torso_up_ok || !result.landmarks.shoulder_axis_ok) {
        return result;
    }

    BlenderShimVec3 x_axis = result.landmarks.shoulder_axis.direction_unit;
    BlenderShimVec3 z_axis = result.landmarks.torso_up.direction_unit;

    int ok_forward = 0;
    BlenderShimVec3 y_axis = normalized_vec3_or_zero(cross_vec3(z_axis, x_axis), &ok_forward);
    if (!ok_forward) {
        return result;
    }

    int ok_x = 0;
    x_axis = normalized_vec3_or_zero(cross_vec3(y_axis, z_axis), &ok_x);
    if (!ok_x) {
        return result;
    }

    int ok_z = 0;
    z_axis = normalized_vec3_or_zero(z_axis, &ok_z);
    if (!ok_z) {
        return result;
    }

    result.basis.x_axis = x_axis;
    result.basis.y_axis = y_axis;
    result.basis.z_axis = z_axis;
    result.basis.ok = 1;

    return result;
}

void blender_shim_debug_print_torso_frame(
    const BlenderShimNamedJoint *joints,
    int joint_count)
{
    const BlenderShimTorsoFrameResult r =
        blender_shim_compute_torso_frame(joints, joint_count);

    std::printf(
        "[blender_shim] torso_frame: basis_ok=%d\n",
        r.basis.ok);

    if (r.landmarks.pelvis_center_ok) {
        std::printf(
            "  origin=(%.6f, %.6f, %.6f)\n",
            r.origin.x, r.origin.y, r.origin.z);
    }

    if (r.basis.ok) {
        std::printf(
            "  x_axis=(%.6f, %.6f, %.6f)\n",
            r.basis.x_axis.x, r.basis.x_axis.y, r.basis.x_axis.z);
        std::printf(
            "  y_axis=(%.6f, %.6f, %.6f)\n",
            r.basis.y_axis.x, r.basis.y_axis.y, r.basis.y_axis.z);
        std::printf(
            "  z_axis=(%.6f, %.6f, %.6f)\n",
            r.basis.z_axis.x, r.basis.z_axis.y, r.basis.z_axis.z);
    }

    std::fflush(stdout);
}



BlenderShimArmatureValidationResult blender_shim_validate_armature_desc(
    const BlenderShimArmatureDesc *armature)
{
    BlenderShimArmatureValidationResult result{};
    result.ok = 1;
    result.first_invalid_bone_index = -1;

    if (armature == nullptr || armature->bones == nullptr || armature->bone_count < 0) {
        result.ok = 0;
        result.first_invalid_bone_index = -1;
        return result;
    }

    for (int i = 0; i < armature->bone_count; ++i) {
        const BlenderShimBoneDesc &bone = armature->bones[i];

        if (bone.parent_index >= armature->bone_count || bone.parent_index < -1) {
            result.ok = 0;
            result.has_invalid_parent = 1;
            result.first_invalid_bone_index = i;
            return result;
        }

        const BlenderShimBoneFromJointsResult shape =
            blender_shim_make_bone_from_joints(bone.head, bone.tail);

        if (!shape.ok) {
            result.ok = 0;
            result.has_degenerate_bone = 1;
            result.first_invalid_bone_index = i;
            return result;
        }
    }

    return result;
}

void blender_shim_debug_print_armature_desc(
    const BlenderShimArmatureDesc *armature)
{
    if (armature == nullptr) {
        std::printf("[blender_shim] armature_desc: <null>\n");
        std::fflush(stdout);
        return;
    }

    std::printf(
        "[blender_shim] armature_desc: bone_count=%d\n",
        armature->bone_count);

    for (int i = 0; i < armature->bone_count; ++i) {
        const BlenderShimBoneDesc &bone = armature->bones[i];
        const char *name = bone.name != nullptr ? bone.name : "<null>";

        const BlenderShimBoneFromJointsResult shape =
            blender_shim_make_bone_from_joints(bone.head, bone.tail);

        std::printf(
            "  bone[%d] name=%s parent=%d "
            "head=(%.6f, %.6f, %.6f) "
            "tail=(%.6f, %.6f, %.6f) "
            "len=%.6f ok=%d\n",
            i,
            name,
            bone.parent_index,
            bone.head.x, bone.head.y, bone.head.z,
            bone.tail.x, bone.tail.y, bone.tail.z,
            shape.length,
            shape.ok);
    }

    const BlenderShimArmatureValidationResult validation =
        blender_shim_validate_armature_desc(armature);

    std::printf(
        "  validation: ok=%d invalid_parent=%d degenerate=%d first_bad=%d\n",
        validation.ok,
        validation.has_invalid_parent,
        validation.has_degenerate_bone,
        validation.first_invalid_bone_index);

    std::fflush(stdout);
}