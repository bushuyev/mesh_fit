use anyhow::{Context, Error, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use ndarray::{s, Array3, Array4, Axis};
use ort::{
    session::Session,
    value::{TensorRef, Value},
};
use std::path::{Path, PathBuf};
use crate::onnx::rtm_w3d::BBox;

/// RTMDet exports are often 640x640, but use whatever your ONNX export actually expects.
pub(crate) const INPUT_W: u32 = 640;
pub(crate) const INPUT_H: u32 = 640;


pub(crate) fn preprocess_image(img:&DynamicImage, input_w: u32, input_h: u32) -> Result<(Array4<f32>, f32, f32)> {

    let (orig_w, orig_h) = img.dimensions();
    let rgb = img.to_rgb8();
    let resized = DynamicImage::ImageRgb8(rgb)
        .resize_exact(input_w, input_h, FilterType::CatmullRom)
        .to_rgb8();

    let mut chw = Array3::<f32>::zeros((3, input_h as usize, input_w as usize));

    for y in 0..input_h {
        for x in 0..input_w {
            let p = resized.get_pixel(x, y);
            // RGB, normalized to [0, 1]
            chw[[0, y as usize, x as usize]] = p[0] as f32 / 255.0;
            chw[[1, y as usize, x as usize]] = p[1] as f32 / 255.0;
            chw[[2, y as usize, x as usize]] = p[2] as f32 / 255.0;
        }
    }

    let nchw = chw.insert_axis(Axis(0)); // [1, 3, H, W]
    let scale_x = orig_w as f32 / input_w as f32;
    let scale_y = orig_h as f32 / input_h as f32;

    Ok((nchw, scale_x, scale_y))
}

pub(crate) fn print_value_summary(value: &Value) -> Result<()> {
    // Try float output first.
    if let Ok(arr) = value.try_extract_array::<f32>() {
        println!("  f32 tensor, shape={:?}", arr.shape());
        let flat: Vec<f32> = arr.iter().take(16).copied().collect();
        println!("  first values: {:?}", flat);
        return Ok(());
    }

    if let Ok(arr) = value.try_extract_array::<i64>() {
        println!("  i64 tensor, shape={:?}", arr.shape());
        let flat: Vec<i64> = arr.iter().take(16).copied().collect();
        println!("  first values: {:?}", flat);
        return Ok(());
    }

    if let Ok((shape, data)) = value.try_extract_tensor::<f32>() {
        println!("  f32 tensor, shape={:?}", &**shape);
        let shown = data.iter().take(16).copied().collect::<Vec<_>>();
        println!("  first values: {:?}", shown);
        return Ok(());
    }

    println!("  unsupported or non-tensor output");
    Ok(())
}

/// Heuristic decoder for common object detection exports.
/// Some exports return:
/// - boxes:   [N, num_boxes, 4]
/// - scores:  [N, num_boxes, num_classes]
/// Others return a single packed tensor.
/// This function just handles a few common cases and prints top detections.
pub(crate) fn try_decode_common_detection_outputs(
    outputs: &ort::session::SessionOutputs<'_>,
    scale_x: f32,
    scale_y: f32,
) -> Result<BBox> {
    let mut float_outputs = Vec::new();

    for (name, value) in outputs.iter() {
        if let Ok(arr) = value.try_extract_array::<f32>() {
            float_outputs.push((name.to_string(), arr.to_owned()));
        }
    }

    if float_outputs.is_empty() {
        return Err(Error::msg("no matches"));
    }

    println!("\nAttempting simple detection decode...");
    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }
    // Case 1: one tensor [1, num_boxes, 5] or [1, num_boxes, 6]

    for (name, arr) in &float_outputs {
        let mut best_match = None::<Vec<f32>>;
        if arr.ndim() == 3 && arr.shape()[0] == 1 {
            let last = arr.shape()[2];
            if last == 5 || last == 6 {
                let dets = arr.index_axis(Axis(0), 0);
                println!("Heuristic: treating '{name}' as packed detection tensor. dets.len={}", dets.len());

                for ch in 0..6 {
                    let mut mn = f32::INFINITY;
                    let mut mx = f32::NEG_INFINITY;
                    for row in dets.outer_iter() {
                        let v = row[ch];
                        mn = mn.min(v);
                        mx = mx.max(v);
                    }
                    println!("channel {ch}: min={mn:.6}, max={mx:.6}");
                }

                for (i, row) in dets.outer_iter()/*.take(20)*/.enumerate() {

                    let vals: Vec<f32> = row.iter().copied().collect();
                    // if vals.len() < 5 {
                    //     println!("Heuristic: skipping {} len", vals.len());
                    //     continue;
                    // }
                    if best_match.as_ref().map(|v|v[5] < vals[5]).unwrap_or(true) {
                        best_match.replace(vals);
                    }


                }
                if let Some(vals) = best_match {
                    let score = vals[5];


                    let x1 = vals[0] * scale_x;
                    let y1 = vals[1] * scale_y;
                    let x2 = vals[2] * scale_x;
                    let y2 = vals[3] * scale_y;
                    let cls = if vals.len() >= 6 { vals[5] as i32 } else { 0 };

                    println!(
                        " best match det: cls={cls} score={:.3} box=[{:.1}, {:.1}, {:.1}, {:.1}]",
                        score, x1, y1, x2, y2
                    );

                    return Ok(BBox{
                        x1,
                        y1,
                        x2,
                        y2,
                    })
                }
            }
        }
    }



    Err(Error::msg("no matches"))
}

mod tests {
    #[test]
    fn test_0(){

    }

}

pub fn run_rtm_det(model_path:PathBuf, img:&DynamicImage) -> anyhow::Result<BBox> {
    println!("_run_onnx");

    if let Ok(dylib_path) = std::env::var("ORT_DYLIB_PATH") {
        let _was_set = ort::init_from(dylib_path)
            .context("failed to create ONNX Runtime environment from ORT_DYLIB_PATH")?
            .commit();
    } else {
        let _was_set = ort::init().commit();
    }

    println!("ort init done");

    let mut session = Session::builder()
        .context("Session::builder failed")?
        .commit_from_file(&model_path)
        .with_context(|| format!("failed to load ONNX model: {model_path:?}"))?;

    println!("Loaded model: {model_path:?}");

    println!("Inputs:");
    for (i, input) in session.inputs().iter().enumerate() {
        println!("  [{i}] input='{input:?}'" );
    }

    println!("Outputs:");
    for (i, output) in session.outputs().iter().enumerate() {
        println!("  [{i}] output='{output:?}'", );
    }

    let (input_tensor, scale_x, scale_y) = preprocess_image(img, INPUT_W, INPUT_H)?;
    println!("Input tensor shape: {:?}", input_tensor.dim());

    let outputs = session.run(ort::inputs![
        TensorRef::from_array_view(&input_tensor).context("failed to create input tensor")?
    ])?;

    println!("Got {} outputs", outputs.len());

    for (name, value) in outputs.iter() {
        println!("Output '{name}':");
        print_value_summary(&*value)?;
    }

    try_decode_common_detection_outputs(&outputs, scale_x, scale_y)
}