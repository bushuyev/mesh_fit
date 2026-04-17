use std::path::PathBuf;
use anyhow::Context;
use clap::Args;
use ort::{
    session::Session,
    value::{TensorRef, Value},
};
use crate::onnx::rtmw3d_armature::make_rtmw3d_armature;
use blender_shims::write_armature_desc_to_blend;

pub mod rtm_det;
pub mod rtm_w3d;
mod rtmw3d_armature;

#[derive(Args, Debug)]
pub struct OnnxArgs {
    #[arg(long)]
    det_model_path: PathBuf,

    #[arg(long)]
    w3d_model_path: PathBuf,

    #[arg(long)]
    image_path: PathBuf,
}


pub  fn run_onnx(args:OnnxArgs) -> anyhow::Result<()> {
    let img = image::open(&args.image_path).with_context(|| {
        format!("failed to open image: {:?}", args.image_path)
    })?;

    let bbox = rtm_det::run_rtm_det(args.det_model_path, &img)?;
    let key_points = rtm_w3d::run_rtm_w3d(args.w3d_model_path, &img, bbox)?;
    let armature = make_rtmw3d_armature(&key_points);

    let path = std::env::temp_dir().join("blender_shims_simple_torso_integration.blend");
    write_armature_desc_to_blend(&armature, &path).unwrap();


    Ok(())
}
