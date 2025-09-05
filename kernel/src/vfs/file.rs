use core::sync::atomic::AtomicU64;

use spin::Mutex;

use crate::vfs::inode::InodeRef;

/// Represents an open file handle for a process.
/// It contains a reference to the inode and the current read/write offset.
/// This should probably be moved to a more central file like `fs/file.rs`.
pub struct File {
    pub inode: InodeRef,
    pub offset: AtomicU64,
}
