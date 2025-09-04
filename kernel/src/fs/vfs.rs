use crate::fs::{
    error::FsError,
    file::FileDescriptor,
    filesystem::FilesystemRef,
    inode::{Inode, InodeRef},
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    // The global mount table for the entire OS
    static ref MOUNT_TABLE: Mutex<MountTable> = Mutex::new(MountTable::new());
}

struct Mount {
    path: String,
    fs: FilesystemRef,
}

pub struct MountTable {
    // A simple list of mounts. For performance, a tree or hash map is better.
    mounts: Vec<Mount>,
}

impl MountTable {
    pub fn new() -> Self {
        MountTable { mounts: Vec::new() }
    }

    fn mount(&mut self, path: &str, fs: FilesystemRef) {
        self.mounts.push(Mount {
            path: String::from(path),
            fs,
        });
        // Sort by path length, longest first, to handle nested mounts correctly
        self.mounts.sort_by_key(|m| -(m.path.len() as isize));
    }

    // Given a full path, find the filesystem it belongs to
    fn find_fs<'a>(&self, path: &'a str) -> Option<(&Mount, &'a str)> {
        for mount in &self.mounts {
            let mount_path = mount.path.as_str();
            if path.starts_with(mount_path) {
                let mount_len = mount_path.len();
                // Check for exact match or path separator to avoid partial matches (e.g., /foo matching /foobar)
                if path.len() == mount_len {
                    // Exact match, subpath is empty, representing the root of the mount
                    return Some((mount, ""));
                }
                // Handle root mount separately
                if mount_path == "/" {
                    return Some((mount, &path[1..]));
                }
                if path.as_bytes()[mount_len] == b'/' {
                    // The subpath starts after the mountpoint and the slash
                    return Some((mount, &path[mount_len + 1..]));
                }
            }
        }
        None
    }
}

/// Mounts a filesystem at a given path
pub fn mount(path: &str, fs: FilesystemRef) {
    MOUNT_TABLE.lock().mount(path, fs);
}

/// Resolves a path to an inode
///
/// This is a helper function that traverses the VFS from the root
/// to find the inode corresponding to a given path
fn resolve_path(path_str: &str) -> Result<InodeRef, FsError> {
    if !path_str.starts_with('/') {
        return Err(FsError::InvalidPath);
    }

    let mtable = MOUNT_TABLE.lock();
    let (mount, subpath) = mtable
        .find_fs(path_str)
        .ok_or(FsError::MountPointNotFound)?;

    println!("{:?} - {}", mount.path, subpath);

    let mut current_inode = mount.fs.root()?;

    // Traverse the path components
    // Filter out empty components that can result from trailing or consecutive slashes
    for component in subpath.split('/').filter(|&c| !c.is_empty()) {
        // To find the next inode, we need to look up the component name
        // in the current directory inode.
        current_inode = current_inode.lookup(component)?;
    }

    Ok(current_inode)
}

/// Opens a file by path and returns a file descriptor
/// This is the primary "open" syscall implementation
pub fn open(path: &str) -> Result<Arc<FileDescriptor>, FsError> {
    let inode = resolve_path(path)?;
    Ok(Arc::new(FileDescriptor {
        inode,
        offset: Mutex::new(0),
    }))
}

/// Reads from an open file descriptor, advancing its offset
pub fn read(fd: &FileDescriptor, buf: &mut [u8]) -> Result<usize, FsError> {
    let mut offset = fd.offset.lock();
    let bytes_read = fd.inode.read_at(*offset, buf)?;
    *offset += bytes_read as u64;
    Ok(bytes_read)
}

/// Writes to an open file descriptor, advancing its offset
pub fn write(fd: &FileDescriptor, buf: &[u8]) -> Result<usize, FsError> {
    let mut offset = fd.offset.lock();
    let bytes_written = fd.inode.write_at(*offset, buf)?;
    *offset += bytes_written as u64;
    Ok(bytes_written)
}
