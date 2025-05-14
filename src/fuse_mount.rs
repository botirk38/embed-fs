use fuser::{Filesystem, Request, ReplyAttr, ReplyEntry, ReplyData, ReplyDirectory, FileAttr, FileType};
use libc::{ENOENT};
use std::ffi::OsStr;
use std::path::{PathBuf};
use std::time::{SystemTime, Duration};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};

const TTL: Duration = Duration::from_secs(1);

pub struct VectorFS {
    root: PathBuf,
}

impl VectorFS {
    pub fn new(root: PathBuf) -> Self {
        VectorFS { root }
    }

    fn real_path(&self, path: &str) -> PathBuf {
        let mut full = self.root.clone();
        full.push(path.trim_start_matches('/'));
        full
    }
}

impl Filesystem for VectorFS {
    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        let full_path = self.real_path(name.to_str().unwrap());
        match fs::metadata(&full_path) {
            Ok(meta) => {
                let attr = FileAttr {
                    ino: meta.ino(),
                    size: meta.len(),
                    blocks: meta.blocks(),
                    atime: SystemTime::now(),
                    mtime: SystemTime::now(),
                    ctime: SystemTime::now(),
                    crtime: SystemTime::now(),
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: meta.uid(),
                    gid: meta.gid(),
                    rdev: 0,
                    flags: 0,
                    blksize: 512,
                };
                reply.entry(&TTL, &attr, 0);
            }
            Err(_) => {
                reply.error(ENOENT);
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        // Passthrough not fully implemented — placeholder
        reply.error(ENOENT);
    }
}
