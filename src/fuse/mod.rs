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
    let now = std::time::SystemTime::now();
    FileAttr {
        ino,
        size,
        blocks: (size + 511) / 512,
        atime: now,
        mtime: now,
        ctime: now,
        crtime: now,
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

pub fn create_file_attr_with_times(
    ino: u64, 
    size: u64, 
    file_type: FileType, 
    perm: u16,
    atime: std::time::SystemTime,
    mtime: std::time::SystemTime,
    ctime: std::time::SystemTime,
    crtime: std::time::SystemTime,
) -> FileAttr {
    FileAttr {
        ino,
        size,
        blocks: (size + 511) / 512,
        atime,
        mtime,
        ctime,
        crtime,
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