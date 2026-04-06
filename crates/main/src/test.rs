use anyhow::{Result, bail};
use candle::{DType, Device, Tensor};
use clap::Args;

/// CLI arguments for a minimal Candle tensor test.
#[derive(Args, Debug)]
pub struct TestArgs {
    /// Run the test on CPU instead of CUDA.
    #[arg(long, default_value_t = false)]
    pub cpu: bool,
    /// CUDA device index to use.
    #[arg(long, default_value_t = 0)]
    pub cuda_device: usize,
    /// Test case to run (`basic` or `sam-preprocess`).
    #[arg(long, default_value = "basic")]
    pub case: String,
}

/// Runs a basic Candle random-matmul test on CUDA (or CPU).
pub fn run(args: &TestArgs) -> Result<()> {
    let device = if args.cpu {
        Device::Cpu
    } else {
        Device::new_cuda(args.cuda_device)?
    };

    match args.case.as_str() {
        "basic" => run_basic_matmul(&device),
        "sam-preprocess" => run_sam_preprocess_probe(&device),
        other => bail!("unknown test case: {other}"),
    }
}

fn run_basic_matmul(device: &Device) -> Result<()> {
    let a = Tensor::randn(0f32, 1., (2, 3), device)?;
    let b = Tensor::randn(0f32, 1., (3, 4), device)?;
    let c = a.matmul(&b)?;

    let (m, n) = c.dims2()?;
    if m != 2 || n != 4 {
        bail!("unexpected output shape: ({m}, {n})");
    }
    println!("{c}");
    Ok(())
}

fn run_sam_preprocess_probe(device: &Device) -> Result<()> {
    println!("probe: building cpu u8 image tensor");
    let cpu_img = Tensor::rand(0f32, 255., (3, 128, 128), &Device::Cpu)?.to_dtype(DType::U8)?;

    println!("probe: move image cpu->cuda");
    let gpu_img = cpu_img.to_device(device)?;

    println!("probe: to_dtype(U8->F32) on cuda");
    let gpu_img_f32 = gpu_img.to_dtype(DType::F32)?;

    let pixel_mean = Tensor::new(&[[[123.675f32]], [[116.28f32]], [[103.53f32]]], device)?;
    let pixel_std = Tensor::new(&[[[58.395f32]], [[57.12f32]], [[57.375f32]]], device)?;

    println!("probe: broadcast_sub / broadcast_div");
    let norm = gpu_img_f32
        .broadcast_sub(&pixel_mean)?
        .broadcast_div(&pixel_std)?;

    println!("probe: pad_with_zeros to SAM image size");
    let (_c, h, w) = norm.dims3()?;
    let padded = norm
        .pad_with_zeros(1, 0, 1024 - h)?
        .pad_with_zeros(2, 0, 1024 - w)?;

    println!("probe: final shape {:?}", padded.shape().dims());
    Ok(())
}
