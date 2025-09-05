use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::vfs::{inode::InodeRef, FsError};

/// Represents an open file handle for a process.
/// It contains a reference to the inode and the current read/write offset.
/// This should probably be moved to a more central file like `fs/file.rs`.
pub struct File {
    pub inode: InodeRef,
    pub offset: AtomicU64,
}

impl File {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FsError> {
        let pos = self.offset.load(Ordering::Relaxed);
        let bytes_read = self.inode.read_at(pos, buf)?;
        if bytes_read > 0 {
            self.offset.fetch_add(bytes_read as u64, Ordering::Relaxed);
        }
        Ok(bytes_read)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, FsError> {
        let pos = self.offset.load(Ordering::Relaxed);
        let bytes_written = self.inode.write_at(pos, buf)?;

        if bytes_written > 0 {
            self.offset
                .fetch_add(bytes_written as u64, Ordering::Relaxed);
        }

        Ok(bytes_written)
    }
}
