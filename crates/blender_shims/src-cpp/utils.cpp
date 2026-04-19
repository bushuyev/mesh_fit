//
// Created by bu on 4/18/26.
//
#include <cstdio>
#include "BLI_math_vector.h"
#include "BKE_blender_version.h"

#include <vector>


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


using namespace blender;

extern "C" {
namespace {
    std::once_flag g_init_once;
}

void blender_shim_global_init(const char *argv0)
{
    std::call_once(g_init_once, [arg = std::string(argv0 ? argv0 : "")]() {
        CLG_init();
        CLG_output_use_timestamp_set(true);
        CLG_output_use_memory_set(false);
        CLG_output_use_source_set(false);
        CLG_output_use_basename_set(false);
        BKE_appdir_program_path_init(arg.c_str());
        DNA_sdna_current_init();
        BKE_blender_globals_init();
        BKE_idtype_init();
        BKE_callback_global_init();

        BLI_threadapi_init();


        IMB_init();
        // BKE_images_init();          // if available in your Blender version
        // BKE_node_system_init();
    });
}


float blender_shim_normalize_vec3(const float in[3], float out[3]) {
    out[0] = in[0];
    out[1] = in[1];
    out[2] = in[2];
    return normalize_v3(out);
}

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

float blender_shim_dot_vec3(const float a[3], const float b[3]) {
    return dot_v3v3(a, b);
}
}