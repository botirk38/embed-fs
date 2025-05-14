use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request,
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::fs::{self, File, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

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
                Ok(meta) => {
                    reply.attr(&TTL, &fileattr_from_metadata(1, &meta));
                }
                Err(_) => reply.error(ENOENT),
            }
        } else {
            let entries = fs::read_dir(&self.root);
            if let Ok(entries) = entries {
                for entry in entries.flatten() {
                    let meta = entry.metadata().unwrap();
                    if meta.ino() == ino {
                        reply.attr(&TTL, &fileattr_from_metadata(ino, &meta));
                        return;
                    }
                }
            }
            reply.error(ENOENT);
        }
    }

    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        let full_path = self.real_path(name);
        match fs::metadata(&full_path) {
            Ok(meta) => {
                let attr = fileattr_from_metadata(meta.ino(), &meta);
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
            let meta = entry.metadata().unwrap();
            let kind = if meta.is_dir() {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            let name = entry.file_name();
            idx += 1;
            let _ = reply.add(meta.ino(), idx, kind, name);
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
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if meta.ino() == ino {
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
        atime: meta.modified().unwrap_or(UNIX_EPOCH),
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
