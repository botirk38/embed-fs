use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request,
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::fs::{self, File, Metadata};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use crate::store::load_embedding;
use std::collections::hash_map::DefaultHasher;

const TTL: Duration = Duration::from_secs(1);
const BLOCK_SIZE: u64 = 512;

pub struct EmbedFS {
    root: PathBuf,
}

impl EmbedFS {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }

    fn real_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut full = self.root.clone();
        full.push(path.as_ref());
        full
    }
}

fn inode_for_path(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    hasher.finish()
}

fn inode_for_vector(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{}:vector", path.to_string_lossy()).hash(&mut hasher);
    hasher.finish()
}

fn is_vector_path(path: &Path) -> bool {
    path.extension().is_none_or(|ext| ext == "vector")
}

fn strip_vector_ext(path: &Path) -> Option<PathBuf> {
    let mut base = path.to_path_buf();
    base.set_extension("");
    Some(base)
}

impl Filesystem for EmbedFS {
    fn init(
        &mut self,
        _req: &Request<'_>,
        _config: &mut fuser::KernelConfig,
    ) -> Result<(), libc::c_int> {
        Ok(())
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        if ino == 1 {
            match fs::metadata(&self.root) {
                Ok(meta) => reply.attr(&TTL, &fileattr_from_metadata(1, &meta)),
                Err(_) => reply.error(ENOENT),
            }
            return;
        }

        let entries = fs::read_dir(&self.root);
        if let Ok(entries) = entries {
            for entry in entries.flatten() {
                let path = entry.path();

                // Regular file
                if inode_for_path(&path) == ino {
                    let meta = match fs::metadata(&path) {
                        Ok(m) => m,
                        Err(_) => {
                            reply.error(ENOENT);
                            return;
                        }
                    };
                    let attr = fileattr_from_metadata(ino, &meta);
                    reply.attr(&TTL, &attr);
                    return;
                }

                // .vector virtual file
                if load_embedding(&path).is_ok() {
                    let virtual_path = path.with_extension("vector");
                    if inode_for_vector(&virtual_path) == ino {
                        let vec = load_embedding(&path).unwrap();
                        let size = vec.len() * std::mem::size_of::<f32>();
                        let attr = FileAttr {
                            ino,
                            size: size as u64,
                            blocks: 1,
                            atime: UNIX_EPOCH,
                            mtime: UNIX_EPOCH,
                            ctime: UNIX_EPOCH,
                            crtime: UNIX_EPOCH,
                            kind: FileType::RegularFile,
                            perm: 0o444,
                            nlink: 1,
                            uid: 1000,
                            gid: 1000,
                            rdev: 0,
                            flags: 0,
                            blksize: BLOCK_SIZE as u32,
                        };
                        reply.attr(&TTL, &attr);
                        return;
                    }
                }
            }
        }

        reply.error(ENOENT);
    }

    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        let full_path = self.real_path(name);

        if is_vector_path(&full_path) {
            if let Some(real_path) = strip_vector_ext(&full_path) {
                if fs::metadata(&real_path).is_ok() {
                    if let Ok(vec) = load_embedding(&real_path) {
                        let size = vec.len() * std::mem::size_of::<f32>();
                        let ino = inode_for_vector(&full_path);
                        let attr = FileAttr {
                            ino,
                            size: size as u64,
                            blocks: 1,
                            atime: UNIX_EPOCH,
                            mtime: UNIX_EPOCH,
                            ctime: UNIX_EPOCH,
                            crtime: UNIX_EPOCH,
                            kind: FileType::RegularFile,
                            perm: 0o444,
                            nlink: 1,
                            uid: 1000,
                            gid: 1000,
                            rdev: 0,
                            flags: 0,
                            blksize: BLOCK_SIZE as u32,
                        };
                        reply.entry(&TTL, &attr, 0);
                        return;
                    }
                }
            }
            reply.error(ENOENT);
            return;
        }

        match fs::metadata(&full_path) {
            Ok(meta) => {
                let attr = fileattr_from_metadata(inode_for_path(&full_path), &meta);
                reply.entry(&TTL, &attr, 0);
            }
            Err(_) => reply.error(ENOENT),
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(_) => {
                reply.error(ENOENT);
                return;
            }
        };

        let mut idx: i64 = 0;
        if offset == 0 {
            let _ = reply.add(1, 0, FileType::Directory, ".");
            idx += 1;
            let _ = reply.add(1, idx, FileType::Directory, "..");
            idx += 1;
        }

        for entry in entries.flatten().skip((offset as usize).saturating_sub(2)) {
            let path = entry.path();
            let meta = entry.metadata().unwrap();
            let kind = if meta.is_dir() {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            let name = entry.file_name();
            let ino = inode_for_path(&path);
            idx += 1;
            let _ = reply.add(ino, idx, kind, &name);

            if meta.is_file() && load_embedding(&path).is_ok() {
                let vector_name = name.to_string_lossy().to_string() + ".vector";
                let vector_path = path.with_extension("vector");
                let ino = inode_for_vector(&vector_path);
                idx += 1;
                let _ = reply.add(ino, idx, FileType::RegularFile, vector_name);
            }
        }

        reply.ok();
    }

    fn open(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        reply.opened(0, 0);
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(_) => {
                reply.error(ENOENT);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Check .vector file first
            if let Ok(vec) = load_embedding(&path) {
                let vector_path = path.with_extension("vector");
                if inode_for_vector(&vector_path) == ino {
                    let str_vec = vec
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let bytes = str_vec.into_bytes();
                    let start = offset as usize;
                    let end = (start + size as usize).min(bytes.len());
                    reply.data(&bytes[start..end]);
                    return;
                }
            }

            if inode_for_path(&path) == ino {
                let mut file = match File::open(&path) {
                    Ok(f) => f,
                    Err(_) => {
                        reply.error(ENOENT);
                        return;
                    }
                };

                let mut buf = vec![0; size as usize];
                if file.seek(SeekFrom::Start(offset as u64)).is_err() {
                    reply.error(ENOENT);
                    return;
                }

                let n = file.read(&mut buf).unwrap_or(0);
                reply.data(&buf[..n]);
                return;
            }
        }

        reply.error(ENOENT);
    }
}

fn fileattr_from_metadata(ino: u64, meta: &Metadata) -> FileAttr {
    FileAttr {
        ino,
        size: meta.len(),
        blocks: meta.len() / BLOCK_SIZE,
        atime: meta.accessed().unwrap_or(UNIX_EPOCH),
        mtime: meta.modified().unwrap_or(UNIX_EPOCH),
        ctime: meta.created().unwrap_or(UNIX_EPOCH),
        crtime: UNIX_EPOCH,
        kind: if meta.is_dir() {
            FileType::Directory
        } else {
            FileType::RegularFile
        },
        perm: 0o755,
        nlink: 1,
        uid: meta.uid(),
        gid: meta.gid(),
        rdev: 0,
        flags: 0,
        blksize: BLOCK_SIZE as u32,
    }
}
