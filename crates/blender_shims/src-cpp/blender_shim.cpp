#include "blender_shim.h"
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

using namespace blender;








