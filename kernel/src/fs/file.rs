use spin::Mutex;

use crate::fs::inode::InodeRef;

/// Represents an open file handle for a process.
/// It contains a reference to the inode and the current read/write offset.
/// This should probably be moved to a more central file like `fs/file.rs`.
pub struct FileDescriptor {
    pub inode: InodeRef,
    pub offset: Mutex<u64>,
}
