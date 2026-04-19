//
// Created by bu on 4/18/26.
//
#include <vector>
#include <cstdio>


#include "BLI_math_vector.h"
#include "BLI_threads.h"
#include "BKE_armature.hh"
#include "BKE_collection.hh"
#include "BKE_idtype.hh"
#include "BKE_lib_id.hh"
#include "BKE_main.hh"
#include "BKE_object.hh"


#include "BKE_report.hh"
#include "BKE_scene.hh"

#include "BLO_writefile.hh"

#include "DNA_armature_types.h"
#include "DNA_object_types.h"
#include "DNA_scene_types.h"

#include "ED_armature.hh"
#include "IMB_imbuf.hh"
#include "MEM_guardedalloc.h"
#include "BKE_blender.hh"
#include "BKE_callbacks.hh"
#include "BKE_appdir.hh"
#include "CLG_log.h"
#include "DNA_genfile.h"

#include "utils.h"
#include "armature.h"

using namespace blender;


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

static BlenderShimVec3 midpoint_vec3(BlenderShimVec3 a, BlenderShimVec3 b)
{
    BlenderShimVec3 out{};
    out.x = 0.5f * (a.x + b.x);
    out.y = 0.5f * (a.y + b.y);
    out.z = 0.5f * (a.z + b.z);
    return out;
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

static void blender_shim_set_error(char *out, int out_size, const char *msg)
{
    if (out == nullptr || out_size <= 0) {
        return;
    }
    std::snprintf(out, static_cast<std::size_t>(out_size), "%s", msg ? msg : "unknown error");
}

static void copy_vec3_to_raw(BlenderShimVec3 v, float out[3])
{
     out[0] = v.x;
     out[1] = v.y;
     out[2] = v.z;
}

static Object *blender_shim_create_armature_object_from_desc(
    Main *bmain,
    const BlenderShimArmatureDesc *armature_desc)
{
    bArmature *arm = BKE_armature_add(bmain, "RTMW_Armature");
    Object *obj = BKE_object_add_only_object(bmain, OB_ARMATURE, "RTMW_Armature");
    obj->data = &arm->id;
    id_us_plus(&arm->id);

    ED_armature_to_edit(arm);

    std::vector<EditBone *> created;
    created.resize(armature_desc->bone_count, nullptr);

    for (int i = 0; i < armature_desc->bone_count; ++i) {
        const BlenderShimBoneDesc &src = armature_desc->bones[i];
        const char *name = src.name ? src.name : "Bone";

        EditBone *ebone = ED_armature_ebone_add(arm, name);
        created[i] = ebone;

        copy_vec3_to_raw(src.head, ebone->head);
        copy_vec3_to_raw(src.tail, ebone->tail);
    }

    for (int i = 0; i < armature_desc->bone_count; ++i) {
        const BlenderShimBoneDesc &src = armature_desc->bones[i];
        EditBone *ebone = created[i];

        if (src.parent_index >= 0) {
            ebone->parent = created[src.parent_index];
        }
    }

    ED_armature_from_edit(bmain, arm);
    ED_armature_edit_free(arm);
    BKE_armature_bone_hash_make(arm);

    return obj;
}

BlenderShimWriteBlendResult blender_shim_write_armature_desc_to_blend(
    const BlenderShimArmatureDesc *armature,
    const char *blend_path,
    char *error_out,
    int error_out_size)
{


    BlenderShimWriteBlendResult result{};
    result.ok = 0;

    if (armature == nullptr) {
        blender_shim_set_error(error_out, error_out_size, "armature is null");
        return result;
    }

    if (blend_path == nullptr || blend_path[0] == '\0') {
        blender_shim_set_error(error_out, error_out_size, "blend_path is empty");
        return result;
    }

    fprintf(stderr, "blender_shim_write_armature_desc_to_blend:0 \n");

    const BlenderShimArmatureValidationResult validation = blender_shim_validate_armature_desc(armature);
    if (!validation.ok) {
        blender_shim_set_error(error_out, error_out_size, "armature validation failed");
        return result;
    }

    fprintf(stderr, "blender_shim_write_armature_desc_to_blend: 1\n");

    blender_shim_global_init(nullptr);


    Main *bmain = BKE_main_new();

    Scene *scene = BKE_scene_add(bmain, "Scene");

    Object *obj = blender_shim_create_armature_object_from_desc(bmain, armature);


    BKE_collection_object_add(bmain, scene->master_collection, obj);


    ReportList reports;
    BKE_reports_init(&reports, RPT_STORE);


    BlendFileWriteParams params{};
    const bool ok = BLO_write_file(bmain, blend_path, 0, &params, &reports);


    if (!ok) {
        BKE_reports_free(&reports);
        BKE_main_free(bmain);
        blender_shim_set_error(error_out, error_out_size, "BLO_write_file failed");
        return result;
    }

    BKE_reports_free(&reports);
    BKE_main_free(bmain);

    result.ok = 1;
    blender_shim_set_error(error_out, error_out_size, "");
    return result;
}
