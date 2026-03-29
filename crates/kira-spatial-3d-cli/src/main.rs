use std::path::PathBuf;

use clap::{Parser, Subcommand};
use kira_spatial_3d_cli::run_manifest_path;

#[derive(Parser, Debug)]
#[command(name = "kira-spatial-3d")]
#[command(about = "Manifest-driven deterministic spatial 3D export CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        scalar: bool,
        #[arg(long = "no-contours")]
        no_contours: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Run {
            manifest,
            scalar,
            no_contours,
        } => run_manifest_path(manifest, scalar, no_contours),
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
