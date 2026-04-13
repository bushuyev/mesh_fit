import bpy
import math
from mathutils import Vector
import addon_utils

# ============================================================
# RTMW3D -> Blender armature / Rigify metarig fitter
# ============================================================
#
# Usage:
# 1. Open Blender.
# 2. Scripting tab -> New.
# 3. Paste this script.
# 4. Replace KEYPOINTS_133 with your real RTMW3D frame.
# 5. Run.
#
# By default it creates a SIMPLE armature from the body keypoints.
# If USE_RIGIFY_METARIG = True, it will instead try to create a
# Rigify Basic Human metarig and fit its main body bones.
#
# Notes:
# - RTMW3D gives keypoints, not bone rotations.
# - This script fits a skeleton to the keypoints for a single frame.
# - It is a good first step before animation / retargeting.
# ============================================================

USE_RIGIFY_METARIG = False
DELETE_PREVIOUS = True
OBJECT_NAME = "RTMW_Armature"
METARIG_NAME = "metarig"

# ------------------------------------------------------------
# Replace this with your real 133 points:
# each entry: (x, y, z_rel, score)
# ------------------------------------------------------------
# KEYPOINTS_133 = [(0.0, 0.0, 0.0, 0.0)] * 133

# Example:
# KEYPOINTS_133[0] = (568.4, 337.9, -0.2265, 0.9406)  # nose
# KEYPOINTS_133[1] = (575.2, 332.7, -0.2190, 0.9335)  # left_eye
# ...
KEYPOINTS_133 = [(0.0, 0.0, 0.0, 0.0)] * 133
KEYPOINTS_133[0] = (568.4, 337.9, -0.2265, 0.9406)
KEYPOINTS_133[1] = (575.2, 332.7, -0.2190, 0.9335)
KEYPOINTS_133[2] = (563.3, 332.2, -0.2190, 0.9114)
KEYPOINTS_133[3] = (584.5, 336.8, -0.1284, 0.8660)
KEYPOINTS_133[4] = (557.6, 334.8, -0.1057, 0.8632)
KEYPOINTS_133[5] = (596.9, 377.8, -0.0453, 0.8969)
KEYPOINTS_133[6] = (542.0, 367.9, -0.0529, 0.8619)
KEYPOINTS_133[7] = (599.0, 428.5, -0.1133, 0.8591)
KEYPOINTS_133[8] = (531.7, 421.3, 0.0604, 0.8859)
KEYPOINTS_133[9] = (604.2, 470.5, -0.2114, 0.8188)
KEYPOINTS_133[10] = (536.9, 465.8, 0.0680, 0.8509)
KEYPOINTS_133[11] = (587.1, 467.3, 0.0378, 0.7773)
KEYPOINTS_133[12] = (552.4, 465.8, 0.0378, 0.7790)
KEYPOINTS_133[13] = (574.7, 535.7, 0.0680, 0.9153)
KEYPOINTS_133[14] = (535.3, 538.8, 0.0906, 0.8940)
KEYPOINTS_133[15] = (567.4, 600.9, 0.1057, 0.9142)
KEYPOINTS_133[16] = (523.4, 602.0, 0.1359, 0.9213)
KEYPOINTS_133[17] = (561.7, 621.1, 0.0227, 0.7673)
KEYPOINTS_133[18] = (574.1, 620.1, 0.0755, 0.8130)
KEYPOINTS_133[19] = (565.9, 607.2, 0.1133, 0.7622)
KEYPOINTS_133[20] = (503.2, 618.0, 0.0529, 0.8445)
KEYPOINTS_133[21] = (500.6, 614.4, 0.0982, 0.8352)
KEYPOINTS_133[22] = (526.5, 610.3, 0.2341, 0.7969)
KEYPOINTS_133[23] = (558.1, 332.2, -0.1812, 0.9544)
KEYPOINTS_133[24] = (558.1, 335.8, -0.1812, 0.9633)
KEYPOINTS_133[25] = (558.6, 338.9, -0.1812, 0.9568)
KEYPOINTS_133[26] = (559.1, 342.0, -0.1812, 0.9672)
KEYPOINTS_133[27] = (560.2, 345.1, -0.1737, 0.9733)
KEYPOINTS_133[28] = (561.7, 348.2, -0.1737, 0.9764)
KEYPOINTS_133[29] = (563.8, 350.3, -0.1737, 0.9729)
KEYPOINTS_133[30] = (566.4, 352.4, -0.1737, 0.9713)
KEYPOINTS_133[31] = (569.5, 352.9, -0.1737, 0.9253)
KEYPOINTS_133[32] = (573.1, 352.4, -0.1812, 0.9535)
KEYPOINTS_133[33] = (576.2, 350.8, -0.1888, 0.9540)
KEYPOINTS_133[34] = (578.8, 348.8, -0.1888, 0.9582)
KEYPOINTS_133[35] = (580.4, 346.2, -0.1888, 0.9610)
KEYPOINTS_133[36] = (581.4, 343.1, -0.1888, 0.9515)
KEYPOINTS_133[37] = (582.4, 339.4, -0.1963, 0.9429)
KEYPOINTS_133[38] = (582.9, 336.3, -0.1963, 0.9307)
KEYPOINTS_133[39] = (583.5, 333.2, -0.1963, 0.9514)
KEYPOINTS_133[40] = (559.6, 329.6, -0.2114, 0.9561)
KEYPOINTS_133[41] = (561.2, 328.6, -0.2190, 0.9597)
KEYPOINTS_133[42] = (562.7, 328.6, -0.2190, 0.9727)
KEYPOINTS_133[43] = (564.8, 328.6, -0.2190, 0.9721)
KEYPOINTS_133[44] = (566.4, 329.1, -0.2265, 0.9669)
KEYPOINTS_133[45] = (572.1, 329.6, -0.2341, 0.9702)
KEYPOINTS_133[46] = (574.1, 329.1, -0.2341, 0.9728)
KEYPOINTS_133[47] = (575.7, 329.1, -0.2265, 0.9618)
KEYPOINTS_133[48] = (577.8, 329.1, -0.2265, 0.9703)
KEYPOINTS_133[49] = (579.3, 330.1, -0.2265, 0.9864)
KEYPOINTS_133[50] = (569.0, 332.7, -0.2265, 0.9881)
KEYPOINTS_133[51] = (569.0, 334.8, -0.2341, 0.9827)
KEYPOINTS_133[52] = (568.4, 336.8, -0.2341, 0.9809)
KEYPOINTS_133[53] = (568.4, 338.9, -0.2341, 0.9723)
KEYPOINTS_133[54] = (566.4, 340.5, -0.2190, 0.9987)
KEYPOINTS_133[55] = (567.9, 341.0, -0.2190, 0.9912)
KEYPOINTS_133[56] = (569.0, 341.0, -0.2265, 0.9792)
KEYPOINTS_133[57] = (570.0, 341.0, -0.2265, 0.9768)
KEYPOINTS_133[58] = (571.5, 340.5, -0.2265, 0.9855)
KEYPOINTS_133[59] = (561.7, 332.2, -0.2114, 0.9766)
KEYPOINTS_133[60] = (562.7, 331.7, -0.2190, 0.9812)
KEYPOINTS_133[61] = (564.3, 331.7, -0.2190, 0.9896)
KEYPOINTS_133[62] = (565.9, 332.7, -0.2190, 0.9878)
KEYPOINTS_133[63] = (564.3, 332.7, -0.2190, 0.9981)
KEYPOINTS_133[64] = (562.7, 332.7, -0.2190, 0.9825)
KEYPOINTS_133[65] = (572.6, 332.7, -0.2265, 0.9925)
KEYPOINTS_133[66] = (574.1, 332.2, -0.2265, 0.9840)
KEYPOINTS_133[67] = (576.2, 332.2, -0.2190, 0.9727)
KEYPOINTS_133[68] = (577.8, 332.7, -0.2190, 0.9883)
KEYPOINTS_133[69] = (575.7, 333.2, -0.2190, 0.9906)
KEYPOINTS_133[70] = (574.1, 333.2, -0.2265, 0.9908)
KEYPOINTS_133[71] = (563.8, 344.1, -0.2039, 0.9910)
KEYPOINTS_133[72] = (565.9, 343.6, -0.2114, 0.9913)
KEYPOINTS_133[73] = (567.9, 343.6, -0.2190, 0.9768)
KEYPOINTS_133[74] = (569.0, 343.6, -0.2190, 0.9693)
KEYPOINTS_133[75] = (570.0, 343.6, -0.2190, 0.9610)
KEYPOINTS_133[76] = (572.1, 343.6, -0.2114, 0.9755)
KEYPOINTS_133[77] = (574.7, 344.6, -0.2114, 0.9844)
KEYPOINTS_133[78] = (572.6, 345.7, -0.2114, 0.9678)
KEYPOINTS_133[79] = (570.5, 346.2, -0.2114, 0.9556)
KEYPOINTS_133[80] = (569.0, 346.2, -0.2114, 0.9595)
KEYPOINTS_133[81] = (567.4, 346.2, -0.2114, 0.9718)
KEYPOINTS_133[82] = (565.9, 345.7, -0.2039, 0.9853)
KEYPOINTS_133[83] = (564.8, 344.1, -0.2039, 0.9952)
KEYPOINTS_133[84] = (566.9, 344.1, -0.2114, 0.9751)
KEYPOINTS_133[85] = (569.0, 344.1, -0.2190, 0.9666)
KEYPOINTS_133[86] = (571.0, 344.6, -0.2114, 0.9574)
KEYPOINTS_133[87] = (574.1, 344.6, -0.2114, 0.9882)
KEYPOINTS_133[88] = (571.0, 344.6, -0.2114, 0.9581)
KEYPOINTS_133[89] = (569.0, 344.6, -0.2114, 0.9650)
KEYPOINTS_133[90] = (566.9, 344.6, -0.2039, 0.9738)
KEYPOINTS_133[91] = (605.2, 476.1, -0.1435, 0.8581)
KEYPOINTS_133[92] = (603.1, 479.3, -0.1661, 0.9204)
KEYPOINTS_133[93] = (601.1, 483.9, -0.1661, 0.8984)
KEYPOINTS_133[94] = (600.5, 489.1, -0.1510, 0.8994)
KEYPOINTS_133[95] = (600.0, 493.8, -0.1435, 0.8470)
KEYPOINTS_133[96] = (606.8, 488.6, -0.1435, 0.8547)
KEYPOINTS_133[97] = (605.2, 495.3, -0.1435, 0.8930)
KEYPOINTS_133[98] = (602.6, 499.5, -0.1359, 0.8894)
KEYPOINTS_133[99] = (600.0, 502.6, -0.1435, 0.8782)
KEYPOINTS_133[100] = (608.3, 489.6, -0.1133, 0.8932)
KEYPOINTS_133[101] = (605.2, 495.8, -0.1133, 0.9066)
KEYPOINTS_133[102] = (602.1, 500.0, -0.1133, 0.9001)
KEYPOINTS_133[103] = (598.5, 502.6, -0.1359, 0.8505)
KEYPOINTS_133[104] = (607.8, 490.1, -0.1057, 0.9026)
KEYPOINTS_133[105] = (605.2, 495.3, -0.1057, 0.8876)
KEYPOINTS_133[106] = (602.1, 498.4, -0.0982, 0.8803)
KEYPOINTS_133[107] = (599.0, 500.5, -0.1284, 0.8174)
KEYPOINTS_133[108] = (607.3, 489.6, -0.1057, 0.8544)
KEYPOINTS_133[109] = (604.7, 493.8, -0.1057, 0.8638)
KEYPOINTS_133[110] = (602.1, 495.8, -0.1133, 0.8500)
KEYPOINTS_133[111] = (600.0, 497.4, -0.1284, 0.8161)
KEYPOINTS_133[112] = (537.4, 469.9, -0.1888, 0.9035)
KEYPOINTS_133[113] = (542.5, 473.6, -0.0227, 0.9090)
KEYPOINTS_133[114] = (546.7, 478.7, -0.0227, 0.8956)
KEYPOINTS_133[115] = (547.2, 483.9, -0.0076, 0.8920)
KEYPOINTS_133[116] = (547.2, 489.1, -0.1359, 0.8572)
KEYPOINTS_133[117] = (543.1, 483.4, -0.0302, 0.9257)
KEYPOINTS_133[118] = (544.1, 489.6, -0.0227, 0.9187)
KEYPOINTS_133[119] = (544.6, 493.2, 0.0000, 0.9155)
KEYPOINTS_133[120] = (545.7, 495.8, 0.0227, 0.8979)
KEYPOINTS_133[121] = (538.9, 483.9, -0.0378, 0.9281)
KEYPOINTS_133[122] = (540.0, 490.6, -0.0076, 0.9263)
KEYPOINTS_133[123] = (541.5, 493.8, 0.0151, 0.9168)
KEYPOINTS_133[124] = (542.0, 495.8, 0.0227, 0.8859)
KEYPOINTS_133[125] = (535.8, 483.9, -0.0378, 0.9247)
KEYPOINTS_133[126] = (536.9, 490.1, -0.0151, 0.9227)
KEYPOINTS_133[127] = (538.4, 492.7, 0.0076, 0.8966)
KEYPOINTS_133[128] = (539.4, 494.8, 0.0302, 0.8665)
KEYPOINTS_133[129] = (532.7, 483.9, -0.0529, 0.9117)
KEYPOINTS_133[130] = (533.7, 487.5, -0.0453, 0.8847)
KEYPOINTS_133[131] = (534.8, 489.6, -0.0378, 0.8594)
KEYPOINTS_133[132] = (535.8, 492.2, 0.0529, 0.8470)
# ------------------------------------------------------------
# COCO-WholeBody / RTMW3D body indices
# ------------------------------------------------------------
KP = {
    "nose": 0,
    "left_eye": 1,
    "right_eye": 2,
    "left_ear": 3,
    "right_ear": 4,
    "left_shoulder": 5,
    "right_shoulder": 6,
    "left_elbow": 7,
    "right_elbow": 8,
    "left_wrist": 9,
    "right_wrist": 10,
    "left_hip": 11,
    "right_hip": 12,
    "left_knee": 13,
    "right_knee": 14,
    "left_ankle": 15,
    "right_ankle": 16,
    "left_big_toe": 17,
    "left_small_toe": 18,
    "left_heel": 19,
    "right_big_toe": 20,
    "right_small_toe": 21,
    "right_heel": 22,
}

MIN_SCORE = 0.10

# ------------------------------------------------------------
# Coordinate conversion
# ------------------------------------------------------------
#
# RTMW3D decoded points are usually:
# - x, y in image space
# - z_rel as relative depth-like value
#
# We convert to Blender space as:
#   X = image x (centered)
#   Y = depth from z_rel
#   Z = inverted image y
#
# This is only a visualization / fitting convention.
# You will likely tune SCALE_XY and SCALE_Z.
# ------------------------------------------------------------
SCALE_XY = 0.01
SCALE_Z = 1.0


def clear_object(name: str):
    obj = bpy.data.objects.get(name)
    if obj is not None:
        bpy.data.objects.remove(obj, do_unlink=True)


def ensure_object_mode():
    if bpy.context.object and bpy.context.object.mode != 'OBJECT':
        bpy.ops.object.mode_set(mode='OBJECT')


def midpoint(a: Vector, b: Vector) -> Vector:
    return (a + b) * 0.5


def safe_normalized(v: Vector, fallback: Vector) -> Vector:
    if v.length < 1e-8:
        return fallback.normalized()
    return v.normalized()


def get_raw_point(idx: int):
    if idx < 0 or idx >= len(KEYPOINTS_133):
        return None
    x, y, z, score = KEYPOINTS_133[idx]
    if score < MIN_SCORE:
        return None
    return (float(x), float(y), float(z), float(score))


def compute_image_center():
    xs = []
    ys = []
    for p in KEYPOINTS_133:
        x, y, z, s = p
        if s >= MIN_SCORE:
            xs.append(float(x))
            ys.append(float(y))
    if not xs:
        return 0.0, 0.0
    return sum(xs) / len(xs), sum(ys) / len(ys)


IMG_CX, IMG_CY = compute_image_center()


def to_blender_point(idx: int):
    p = get_raw_point(idx)
    if p is None:
        return None
    x, y, z_rel, _ = p

    bx = (x - IMG_CX) * SCALE_XY
    by = z_rel * SCALE_Z
    bz = -(y - IMG_CY) * SCALE_XY
    return Vector((bx, by, bz))


def choose_point(*names):
    for name in names:
        idx = KP[name]
        p = to_blender_point(idx)
        if p is not None:
            return p
    return None


def infer_body_landmarks():
    pts = {}
    for name, idx in KP.items():
        pts[name] = to_blender_point(idx)

    left_hip = pts["left_hip"]
    right_hip = pts["right_hip"]
    left_shoulder = pts["left_shoulder"]
    right_shoulder = pts["right_shoulder"]
    nose = pts["nose"]

    pelvis = midpoint(left_hip, right_hip) if left_hip and right_hip else None
    chest = midpoint(left_shoulder, right_shoulder) if left_shoulder and right_shoulder else None

    neck = None
    if chest and nose:
        neck = chest.lerp(nose, 0.35)
    elif chest:
        neck = chest + Vector((0.0, 0.0, 0.15))

    head = None
    if nose and neck:
        head = neck + (nose - neck) * 1.25
    elif nose:
        head = nose
    elif neck:
        head = neck + Vector((0.0, 0.0, 0.12))

    spine_mid = None
    if pelvis and chest:
        spine_mid = pelvis.lerp(chest, 0.5)

    # Slight forward bend helper for knees/elbows if needed
    forward = Vector((0.0, 1.0, 0.0))

    def nudge_joint(a, j, b, amount=0.03):
        if a and j and b:
            axis = (b - a)
            side = axis.cross(Vector((0.0, 0.0, 1.0)))
            if side.length < 1e-8:
                return j + forward * amount
            return j + safe_normalized(side.cross(axis), forward) * amount
        return j

    # Rigify likes knees/elbows not perfectly straight.
    if pts["left_elbow"] and pts["left_shoulder"] and pts["left_wrist"]:
        pts["left_elbow"] = nudge_joint(pts["left_shoulder"], pts["left_elbow"], pts["left_wrist"])
    if pts["right_elbow"] and pts["right_shoulder"] and pts["right_wrist"]:
        pts["right_elbow"] = nudge_joint(pts["right_shoulder"], pts["right_elbow"], pts["right_wrist"])
    if pts["left_knee"] and pts["left_hip"] and pts["left_ankle"]:
        pts["left_knee"] = nudge_joint(pts["left_hip"], pts["left_knee"], pts["left_ankle"])
    if pts["right_knee"] and pts["right_hip"] and pts["right_ankle"]:
        pts["right_knee"] = nudge_joint(pts["right_hip"], pts["right_knee"], pts["right_ankle"])

    pts["pelvis_center"] = pelvis
    pts["chest_center"] = chest
    pts["neck_center"] = neck
    pts["head_center"] = head
    pts["spine_mid"] = spine_mid
    return pts


LAND = infer_body_landmarks()


def require(name):
    p = LAND.get(name)
    if p is None:
        raise RuntimeError(f"Missing required keypoint/landmark: {name}")
    return p


# ============================================================
# Simple armature mode
# ============================================================

def add_bone(ebones, name, head: Vector, tail: Vector, parent=None, use_connect=False):
    b = ebones.new(name)
    b.head = head
    b.tail = tail if (tail - head).length > 1e-5 else head + Vector((0.0, 0.0, 0.05))
    if parent is not None:
        b.parent = parent
        b.use_connect = use_connect
    return b


def build_simple_armature():
    ensure_object_mode()

    if DELETE_PREVIOUS:
        clear_object(OBJECT_NAME)

    arm_data = bpy.data.armatures.new(OBJECT_NAME)
    arm_obj = bpy.data.objects.new(OBJECT_NAME, arm_data)
    bpy.context.collection.objects.link(arm_obj)
    bpy.context.view_layer.objects.active = arm_obj
    arm_obj.select_set(True)

    bpy.ops.object.mode_set(mode='EDIT')
    eb = arm_data.edit_bones

    pelvis = require("pelvis_center")
    spine_mid = require("spine_mid")
    chest = require("chest_center")
    neck = require("neck_center")
    head = require("head_center")

    l_sh = require("left_shoulder")
    r_sh = require("right_shoulder")
    l_el = require("left_elbow")
    r_el = require("right_elbow")
    l_wr = require("left_wrist")
    r_wr = require("right_wrist")

    l_hip = require("left_hip")
    r_hip = require("right_hip")
    l_kn = require("left_knee")
    r_kn = require("right_knee")
    l_an = require("left_ankle")
    r_an = require("right_ankle")

    l_toe = choose_point("left_big_toe", "left_small_toe", "left_heel")
    r_toe = choose_point("right_big_toe", "right_small_toe", "right_heel")

    spine = add_bone(eb, "spine", pelvis, spine_mid)
    chest_b = add_bone(eb, "chest", spine_mid, chest, parent=spine, use_connect=True)
    neck_b = add_bone(eb, "neck", chest, neck, parent=chest_b, use_connect=True)
    head_b = add_bone(eb, "head", neck, head, parent=neck_b, use_connect=True)

    ul = add_bone(eb, "upper_arm.L", l_sh, l_el, parent=chest_b, use_connect=False)
    fl = add_bone(eb, "forearm.L", l_el, l_wr, parent=ul, use_connect=True)

    ur = add_bone(eb, "upper_arm.R", r_sh, r_el, parent=chest_b, use_connect=False)
    fr = add_bone(eb, "forearm.R", r_el, r_wr, parent=ur, use_connect=True)

    tl = add_bone(eb, "thigh.L", l_hip, l_kn, parent=spine, use_connect=False)
    sl = add_bone(eb, "shin.L", l_kn, l_an, parent=tl, use_connect=True)

    tr = add_bone(eb, "thigh.R", r_hip, r_kn, parent=spine, use_connect=False)
    sr = add_bone(eb, "shin.R", r_kn, r_an, parent=tr, use_connect=True)

    if l_toe is not None:
        add_bone(eb, "foot.L", l_an, l_toe, parent=sl, use_connect=True)
    if r_toe is not None:
        add_bone(eb, "foot.R", r_an, r_toe, parent=sr, use_connect=True)

    bpy.ops.object.mode_set(mode='OBJECT')
    arm_obj.show_in_front = True
    arm_data.display_type = 'STICK'
    print(f"Created simple armature: {arm_obj.name}")


# ============================================================
# Rigify metarig mode
# ============================================================

def ensure_rigify():
    loaded_default, loaded_state = addon_utils.check("rigify")
    if not loaded_state:
        addon_utils.enable("rigify", default_set=True)


def find_existing_bone(edit_bones, candidates):
    for name in candidates:
        if name in edit_bones:
            return edit_bones[name]
    return None


def print_bone_names(obj):
    print("Metarig bone names:")
    for b in obj.data.bones:
        print("  ", b.name)


def set_bone_head_tail(edit_bones, candidates, head, tail):
    b = find_existing_bone(edit_bones, candidates)
    if b is None:
        print(f"[WARN] Could not find any of bones: {candidates}")
        return None
    b.head = head
    if (tail - head).length < 1e-5:
        tail = head + Vector((0.0, 0.0, 0.05))
    b.tail = tail
    return b


def create_or_get_basic_human_metarig():
    ensure_object_mode()
    ensure_rigify()

    if DELETE_PREVIOUS:
        clear_object(METARIG_NAME)

    # This operator is widely used for the predefined Rigify Basic Human metarig.
    bpy.ops.object.armature_basic_human_metarig_add()
    obj = bpy.context.active_object
    obj.name = METARIG_NAME
    obj.data.name = METARIG_NAME + "_data"
    return obj


def fit_rigify_metarig():
    obj = create_or_get_basic_human_metarig()
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT')
    eb = obj.data.edit_bones

    pelvis = require("pelvis_center")
    spine_mid = require("spine_mid")
    chest = require("chest_center")
    neck = require("neck_center")
    head = require("head_center")

    l_sh = require("left_shoulder")
    r_sh = require("right_shoulder")
    l_el = require("left_elbow")
    r_el = require("right_elbow")
    l_wr = require("left_wrist")
    r_wr = require("right_wrist")

    l_hip = require("left_hip")
    r_hip = require("right_hip")
    l_kn = require("left_knee")
    r_kn = require("right_knee")
    l_an = require("left_ankle")
    r_an = require("right_ankle")

    l_toe = choose_point("left_big_toe", "left_small_toe", "left_heel")
    r_toe = choose_point("right_big_toe", "right_small_toe", "right_heel")

    # Bone names can vary a bit across metarigs / versions.
    # We try common names first and print warnings for missing ones.
    set_bone_head_tail(eb, ["spine", "hips"], pelvis, spine_mid)
    set_bone_head_tail(eb, ["spine.001", "spine.01", "torso"], spine_mid, chest)
    set_bone_head_tail(eb, ["spine.002", "spine.02", "chest"], chest, neck)
    set_bone_head_tail(eb, ["spine.003", "spine.03", "neck"], neck, head)

    set_bone_head_tail(eb, ["shoulder.L"], chest, l_sh)
    set_bone_head_tail(eb, ["upper_arm.L"], l_sh, l_el)
    set_bone_head_tail(eb, ["forearm.L"], l_el, l_wr)

    set_bone_head_tail(eb, ["shoulder.R"], chest, r_sh)
    set_bone_head_tail(eb, ["upper_arm.R"], r_sh, r_el)
    set_bone_head_tail(eb, ["forearm.R"], r_el, r_wr)

    set_bone_head_tail(eb, ["thigh.L"], l_hip, l_kn)
    set_bone_head_tail(eb, ["shin.L"], l_kn, l_an)
    if l_toe is not None:
        set_bone_head_tail(eb, ["foot.L"], l_an, l_toe)

    set_bone_head_tail(eb, ["thigh.R"], r_hip, r_kn)
    set_bone_head_tail(eb, ["shin.R"], r_kn, r_an)
    if r_toe is not None:
        set_bone_head_tail(eb, ["foot.R"], r_an, r_toe)

    bpy.ops.object.mode_set(mode='OBJECT')
    obj.show_in_front = True
    obj.data.display_type = 'STICK'

    print(f"Created and fitted Rigify metarig: {obj.name}")
    print_bone_names(obj)
    print("If some warnings appeared, use the printed bone names to adjust candidate lists in the script.")


# ============================================================
# Optional helper: load points from a compact multiline string
# ============================================================

def parse_points_text(text: str):
    pts = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = [x.strip() for x in line.replace("(", "").replace(")", "").split(",")]
        if len(parts) != 4:
            continue
        pts.append(tuple(float(x) for x in parts))
    return pts


# ============================================================
# Run
# ============================================================

def main():
    required = [
        "left_shoulder", "right_shoulder",
        "left_elbow", "right_elbow",
        "left_wrist", "right_wrist",
        "left_hip", "right_hip",
        "left_knee", "right_knee",
        "left_ankle", "right_ankle",
        "nose",
    ]
    missing = [name for name in required if LAND.get(name) is None]
    if missing:
        raise RuntimeError(
            "Not enough confident keypoints to build rig. Missing: " + ", ".join(missing)
        )

    print(f"USE_RIGIFY_METARIG={USE_RIGIFY_METARIG}")

    if USE_RIGIFY_METARIG:
        fit_rigify_metarig()
    else:
        build_simple_armature()

    bpy.ops.wm.save_as_mainfile(filepath="rtmw_armature.blend")

if __name__ == "__main__":
    main()