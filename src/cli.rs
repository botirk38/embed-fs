use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "VectorFS")]
#[command(about = "Smart filesystem with vector embedding support", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Mount the vector-aware FUSE filesystem
    Mount {
        source: PathBuf,
        mountpoint: PathBuf,
    },
    /// Embed a vector into a file's metadata
    Embed {
        file: PathBuf,
        vector: String, // comma-separated string, e.g., "0.1,0.2,0.3"
    },
    /// Show the stored vector for a file
    Show {
        file: PathBuf,
    },
}
