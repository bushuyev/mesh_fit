use std::path::PathBuf;
use anyhow::Context;
use clap::Args;
use ort::{
    session::Session,
    value::{TensorRef, Value},
};
pub mod rtm_det;
pub mod rtm_w3d;

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
    rtm_w3d::run_rtm_w3d(args.w3d_model_path, &img, bbox)?;

    Ok(())
}
