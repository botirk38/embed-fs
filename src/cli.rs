use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "EmbedFS")]
#[command(about = "Smart filesystem with vector embedding support", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)] // Now works
pub enum Commands {
    Mount {
        source: PathBuf,
        mountpoint: PathBuf,
    },
    Embed {
        file: PathBuf,
        vector: String,
    },
    Show {
        file: PathBuf,
    },
}
