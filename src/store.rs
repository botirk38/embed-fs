use crate::embedding::Embedding;
use sled::Db;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use bincode::config;
use bincode::serde::{decode_from_slice, encode_to_vec};

fn db() -> Db {
    sled::open("vectorfs_db").expect("Failed to open sled DB")
}

fn file_key<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let metadata = fs::metadata(&path)?;
    let dev = metadata.dev();
    let ino = metadata.ino();
    Ok(format!("{}:{}", dev, ino).as_bytes().to_vec())
}

pub fn save_embedding<P: AsRef<Path>>(path: P, vec: &[f32]) -> io::Result<()> {
    let key = file_key(&path)?;
    let emb = Embedding {
        vector: vec.to_vec(),
    };

    let config = config::standard();
    let data = encode_to_vec(&emb, config).unwrap();

    db().insert(key, data).unwrap();
    Ok(())
}

pub fn load_embedding<P: AsRef<Path>>(path: P) -> io::Result<Vec<f32>> {
    let key = file_key(&path)?;
    if let Some(val) = db().get(key).unwrap() {
        let config = config::standard();
        let (emb, _len): (Embedding, _) = decode_from_slice(&val, config).unwrap();
        Ok(emb.vector)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No embedding found",
        ))
    }
}
