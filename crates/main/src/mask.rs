use anyhow::{Context, Result, bail};
use candle::DType;
use candle_nn::VarBuilder;
use candle_transformers::models::segment_anything::sam;
use clap::Args;
use hf_hub::api::sync::Api;
use std::path::{Path, PathBuf};

/// CLI arguments for batch image masking with Segment Anything.
#[derive(Args, Debug)]
pub struct MaskArgs {
    /// Directory containing input images.
    #[arg(long)]
    pub in_dir: PathBuf,
    /// Directory where generated masks are written.
    #[arg(long)]
    pub out_dir: PathBuf,
    /// Force CPU instead of CUDA.
    #[arg(long, default_value_t = false)]
    pub cpu: bool,
    /// Hugging Face repo containing the SAM weights.
    #[arg(long, default_value = "lmz/candle-sam")]
    pub hf_model: String,
    /// Weights filename inside the model repo.
    #[arg(long, default_value = "sam_vit_b_01ec64.safetensors")]
    pub weights: String,
    /// SAM automatic mask generation: points per side.
    #[arg(long, default_value_t = 32)]
    pub points_per_side: usize,
    /// SAM automatic mask generation: crop layers.
    #[arg(long, default_value_t = 0)]
    pub crop_n_layer: usize,
    /// SAM automatic mask generation: crop overlap ratio.
    #[arg(long, default_value_t = 512.0 / 1500.0)]
    pub crop_overlap_ratio: f64,
    /// SAM automatic mask generation: downscale factor.
    #[arg(long, default_value_t = 1)]
    pub crop_n_points_downscale_factor: usize,
}

/// Generates masks for every image under `in_dir` and saves them to `out_dir`.
pub fn run(args: &MaskArgs) -> Result<()> {
    std::fs::create_dir_all(&args.out_dir)?;

    match run_inner(args, args.cpu) {
        Ok(()) => Ok(()),
        Err(err) if !args.cpu && is_cuda_runtime_error(&err) => {
            eprintln!("initial CUDA masking error:\n{err:#}");
            eprintln!(
                "CUDA execution failed (driver/toolchain mismatch or unsupported runtime). \
                 Retrying on CPU. Use --cpu to force this mode explicitly."
            );
            run_inner(args, true).context("masking failed after CPU fallback")
        }
        Err(err) => Err(err),
    }
}

/// Executes masking with an explicit device choice (CPU when `force_cpu=true`).
fn run_inner(args: &MaskArgs, force_cpu: bool) -> Result<()> {
    // Uses CUDA by default when available unless explicitly forced to CPU.
    let device = candle_examples::device(force_cpu)?;

    let api = Api::new()?;
    let repo = api.model(args.hf_model.clone());
    let model_path = repo
        .get(&args.weights)
        .with_context(|| format!("download {:?}", args.weights))?;

    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &device)
            .context("load SAM weights")?
    };
    let sam = sam::Sam::new(768, 12, 12, &[2, 5, 8, 11], vb).context("build SAM model")?;

    let image_paths = collect_images(&args.in_dir)?;
    if image_paths.is_empty() {
        bail!("no image files found in {:?}", args.in_dir);
    }

    for image_path in image_paths {
        process_one_image(&sam, args, &device, &image_path)?;
    }
    Ok(())
}

/// Detects CUDA errors that can be recovered by retrying on CPU.
fn is_cuda_runtime_error(err: &anyhow::Error) -> bool {
    let s = format!("{err:#}").to_ascii_lowercase();
    s.contains("drivererror(")
        || s.contains("cuda_error_")
        || s.contains("cuda ")
        || s.contains("cuda_error_operating_system")
        || s.contains("cuda_error_unsupported_ptx_version")
        || s.contains("unsupported ptx")
        || s.contains("unsupported toolchain")
}

/// Runs SAM automatic mask generation for a single image and writes PNG masks.
fn process_one_image(
    sam: &sam::Sam,
    args: &MaskArgs,
    device: &candle::Device,
    image_path: &Path,
) -> Result<()> {
    let (image, initial_h, initial_w) =
        candle_examples::load_image(image_path, Some(sam::IMAGE_SIZE))
            .with_context(|| format!("load image {:?}", image_path))?;
    let image = image.to_device(device)?;

    let bboxes = sam.generate_masks(
        &image,
        args.points_per_side,
        args.crop_n_layer,
        args.crop_overlap_ratio,
        args.crop_n_points_downscale_factor,
    )?;

    let stem = image_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");

    for (idx, bbox) in bboxes.iter().enumerate() {
        let mask = (&bbox.data.to_dtype(DType::U8)? * 255.)?; // 0/255
        let (h, w) = mask.dims2()?;
        let mask = mask.broadcast_as((3, h, w))?; // RGB for save helper
        let out_file = args.out_dir.join(format!("{stem}_mask_{idx:03}.png"));
        candle_examples::save_image_resize(&mask, &out_file, initial_h, initial_w)
            .with_context(|| format!("write mask {:?}", out_file))?;
    }
    eprintln!("processed {:?}: {} masks", image_path, bboxes.len());
    Ok(())
}

/// Recursively collects supported image files from a directory.
fn collect_images(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        bail!("input directory does not exist: {:?}", root);
    }
    if !root.is_dir() {
        bail!("input path is not a directory: {:?}", root);
    }

    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if is_supported_image(&path) {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(out)
}

/// Returns true for image extensions accepted by this batch tool.
fn is_supported_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "webp")
    )
}
