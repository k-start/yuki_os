use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::fs::error::FsError;

// An enum to represent the type of the inode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeKind {
    File,
    Directory,
    Device,
}

// A directory entry, which contains a name and a reference to the inode
pub struct DirEntry {
    pub name: String,
    pub inode: InodeRef,
}

// The core Inode trait
pub trait Inode: Send + Sync {
    /// Reads data from the inode at a specific offset.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, FsError>;

    /// Writes data to the inode at a specific offset.
    fn write_at(&self, offset: u64, buf: &[u8]) -> Result<usize, FsError>;

    /// Returns the type of the inode (file, directory, etc.).
    fn kind(&self) -> InodeKind;

    /// Returns the size of the file in bytes.
    fn size(&self) -> u64;

    /// Lists entries in a directory
    ///
    /// This should only be implemented for directory inodes
    fn list_entries(&self) -> Result<Vec<DirEntry>, FsError> {
        Err(FsError::NotADirectory)
    }

    /// Looks up a child inode by name in a directory
    ///
    /// This has a default implementation that uses `list_entries`, but can be
    // overridden by filesystems for better performance
    fn lookup(&self, name: &str) -> Result<InodeRef, FsError> {
        if self.kind() != InodeKind::Directory {
            return Err(FsError::NotADirectory);
        }
        for entry in self.list_entries()? {
            if entry.name == name {
                return Ok(entry.inode);
            }
        }
        Err(FsError::NotFound)
    }
}

// We'll almost always pass inodes around as thread-safe reference-counted pointers.
pub type InodeRef = Arc<dyn Inode>;
