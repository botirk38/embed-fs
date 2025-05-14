mod fuse_mount;
mod store;
mod embedding;
mod cli;

use clap::Parser;
use cli::Cli;
use fuse_mount::VectorFS;

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(cli::Commands::Mount { source, mountpoint }) => {
            let fs = VectorFS::new(source.into());
            fuser::mount2(fs, mountpoint, &[]).expect("Failed to mount filesystem");
        }
        Some(cli::Commands::Embed { file, vector }) => {
            let vec: Vec<f32> = vector.split(',')
                .filter_map(|s| s.parse().ok())
                .collect();

            store::save_embedding(file, &vec).expect("Failed to save embedding");
        }
        Some(cli::Commands::Show { file }) => {
            match store::load_embedding(file) {
                Ok(vec) => println!("{:?}", vec),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }
}
