use anyhow::Result;
use clap::{Parser, Subcommand};

mod mask;
mod train;

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
}

/// Program entrypoint: parse args and dispatch to the selected mode.
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Train(args) => train::run(args),
        Command::Mask(args) => mask::run(&args),
    }
}
