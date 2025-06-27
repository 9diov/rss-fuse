pub mod filesystem;
pub mod inode;
pub mod operations;

use fuser::{FileAttr, FileType};
use libc::{ENOENT, ENOTDIR};
use std::time::{Duration, UNIX_EPOCH};

pub use filesystem::RssFuseFilesystem;
pub use inode::{InodeManager, NodeType};
pub use operations::{FuseOperations, MountOptions, FuseStats};

pub const TTL: Duration = Duration::from_secs(1);

pub fn create_file_attr(ino: u64, size: u64, file_type: FileType, perm: u16) -> FileAttr {
    FileAttr {
        ino,
        size,
        blocks: (size + 511) / 512,
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind: file_type,
        perm,
        nlink: if file_type == FileType::Directory { 2 } else { 1 },
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
        rdev: 0,
        flags: 0,
        blksize: 4096,
    }
}