mod cli;
mod embedding;
mod fuse_mount;
mod store;

use clap::Parser;
use cli::Cli;
use fuse_mount::EmbedFS;

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(cli::Commands::Mount { source, mountpoint }) => {
            let fs = EmbedFS::new(source);
            fuser::mount2(fs, mountpoint, &[]).expect("Failed to mount filesystem");
        }
        Some(cli::Commands::Embed { file, vector }) => {
            let vec = if let Some(text) = vector {
                text.split(',')
                    .filter_map(|s| s.trim().parse::<f32>().ok())
                    .collect::<Vec<f32>>()
            } else {
                store::generate_embedding(file).expect("Failed to generate embedding")
            };

            store::save_embedding(file, &vec).expect("Failed to save embedding");
        }
        Some(cli::Commands::Show { file }) => match store::load_embedding(file) {
            Ok(vec) => println!("{:?}", vec),
            Err(e) => eprintln!("Error: {}", e),
        },
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }
}
