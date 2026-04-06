use anyhow::{Context, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use ndarray::{s, Array3, Array4, Axis};
use ort::{
    session::Session,
    value::{TensorRef, Value},
};
use std::path::Path;

/// RTMDet exports are often 640x640, but use whatever your ONNX export actually expects.
const INPUT_W: u32 = 640;
const INPUT_H: u32 = 640;

fn main() -> Result<()> {
    if let Ok(dylib_path) = std::env::var("ORT_DYLIB_PATH") {
        let _was_set = ort::init_from(dylib_path)
            .context("failed to create ONNX Runtime environment from ORT_DYLIB_PATH")?
            .commit();
    } else {
        let _was_set = ort::init().commit();
    }

    let model_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "rtmdet.onnx".to_string());
    let image_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "input.jpg".to_string());

    let mut session = Session::builder()
        .context("Session::builder failed")?
        .commit_from_file(&model_path)
        .with_context(|| format!("failed to load ONNX model: {model_path}"))?;

    println!("Loaded model: {model_path}");

    println!("Inputs:");
    for (i, input) in session.inputs().iter().enumerate() {
        println!("  [{i}] input='{input:?}'" );
    }

    println!("Outputs:");
    for (i, output) in session.outputs().iter().enumerate() {
        println!("  [{i}] output='{output:?}'", );
    }

    let (input_tensor, scale_x, scale_y) = preprocess_image(&image_path, INPUT_W, INPUT_H)?;
    println!("Input tensor shape: {:?}", input_tensor.dim());

    let outputs = session.run(ort::inputs![
        TensorRef::from_array_view(&input_tensor).context("failed to create input tensor")?
    ])?;

    println!("Got {} outputs", outputs.len());

    for (name, value) in outputs.iter() {
        println!("Output '{name}':");
        print_value_summary(&*value)?;
    }

    try_decode_common_detection_outputs(&outputs, scale_x, scale_y)?;
    Ok(())
}

fn preprocess_image<P: AsRef<Path>>(path: P, input_w: u32, input_h: u32) -> Result<(Array4<f32>, f32, f32)> {
    let img = image::open(&path).with_context(|| {
        format!("failed to open image: {}", path.as_ref().display())
    })?;

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

fn print_value_summary(value: &Value) -> Result<()> {
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
fn try_decode_common_detection_outputs(
    outputs: &ort::session::SessionOutputs<'_>,
    scale_x: f32,
    scale_y: f32,
) -> Result<()> {
    let mut float_outputs = Vec::new();

    for (name, value) in outputs.iter() {
        if let Ok(arr) = value.try_extract_array::<f32>() {
            float_outputs.push((name.to_string(), arr.to_owned()));
        }
    }

    if float_outputs.is_empty() {
        return Ok(());
    }

    println!("\nAttempting simple detection decode...");

    // Case 1: one tensor [1, num_boxes, 5] or [1, num_boxes, 6]
    for (name, arr) in &float_outputs {
        if arr.ndim() == 3 && arr.shape()[0] == 1 {
            let last = arr.shape()[2];
            if last == 5 || last == 6 {
                println!("Heuristic: treating '{name}' as packed detection tensor.");
                let dets = arr.index_axis(Axis(0), 0);

                for (i, row) in dets.outer_iter().take(20).enumerate() {
                    let vals: Vec<f32> = row.iter().copied().collect();
                    if vals.len() < 5 {
                        continue;
                    }

                    let score = vals[4];
                    if score < 0.25 {
                        continue;
                    }

                    let x1 = vals[0] * scale_x;
                    let y1 = vals[1] * scale_y;
                    let x2 = vals[2] * scale_x;
                    let y2 = vals[3] * scale_y;
                    let cls = if vals.len() >= 6 { vals[5] as i32 } else { 0 };

                    println!(
                        "  det #{i}: cls={cls} score={:.3} box=[{:.1}, {:.1}, {:.1}, {:.1}]",
                        score, x1, y1, x2, y2
                    );
                }
                return Ok(());
            }
        }
    }

    // Case 2: boxes [1, B, 4] + scores [1, B, C]
    let boxes = float_outputs
        .iter()
        .find(|(_, arr)| arr.ndim() == 3 && arr.shape()[0] == 1 && arr.shape()[2] == 4);
    let scores = float_outputs
        .iter()
        .find(|(_, arr)| arr.ndim() == 3 && arr.shape()[0] == 1 && arr.shape()[1] > 0 && arr.shape()[2] > 1);

    if let (Some((boxes_name, boxes)), Some((scores_name, scores))) = (boxes, scores) {
        println!("Heuristic: treating '{boxes_name}' as boxes and '{scores_name}' as scores.");

        let boxes = boxes.index_axis(Axis(0), 0);   // [B, 4]
        let scores = scores.index_axis(Axis(0), 0); // [B, C]

        for i in 0..boxes.shape()[0].min(50) {
            let score_row = scores.slice(s![i, ..]);
            let (best_cls, best_score) = score_row
                .iter()
                .copied()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .unwrap();

            if best_score < 0.25 {
                continue;
            }

            let b = boxes.slice(s![i, ..]);
            let x1 = b[0] * scale_x;
            let y1 = b[1] * scale_y;
            let x2 = b[2] * scale_x;
            let y2 = b[3] * scale_y;

            println!(
                "  det #{i}: cls={} score={:.3} box=[{:.1}, {:.1}, {:.1}, {:.1}]",
                best_cls, best_score, x1, y1, x2, y2
            );
        }
    }

    Ok(())
}

mod tests {
    #[test]
    fn test_0(){
        
    }

}