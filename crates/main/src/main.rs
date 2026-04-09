use anyhow::Result;
use clap::{Parser, Subcommand};
use crate::mod_net::ModArgs;
use crate::onnx::OnnxArgs;

mod mask;
mod test;
mod train;
pub mod mod_net;
pub mod onnx;

/// Top-level CLI with subcommands for training and masking.
#[derive(Parser, Debug)]
#[command(about = "Mesh fitting training and image masking utilities")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Supported executable modes.
#[derive(Subcommand, Debug)]
enum Command {
    /// Runs mesh joint-scale fitting against SDF views.
    Train(train::TrainArgs),
    /// Runs SAM automatic masking for images in a directory.
    Mask(mask::MaskArgs),
    /// Runs a minimal Candle tensor matmul test on CUDA (or CPU).
    Test(test::TestArgs),

    Onnx(OnnxArgs),
}

/// Program entrypoint: parse args and dispatch to the selected mode.
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Train(args) => train::run(args),
        Command::Mask(args) => mask::run(&args),
        Command::Test(args) => test::run(&args),
        Command::Onnx(args) => onnx::run_onnx(args)
    }
}
