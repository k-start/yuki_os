use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use alloc::{collections::btree_map::BTreeMap, sync::Arc};
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

pub struct FileDescriptorTable {
    /// The map from the file descriptor number (usize) to the File object.
    files: BTreeMap<usize, Arc<File>>,

    /// The next available file descriptor number to try.
    next_fd: AtomicUsize,
}

impl FileDescriptorTable {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            next_fd: AtomicUsize::new(0),
        }
    }

    /// Creates a new entry in the table for the given File object.
    pub fn add(&mut self, file: Arc<File>) -> usize {
        // Find the next available file descriptor ID
        let mut fd = self.next_fd.load(Ordering::Relaxed);
        while self.files.contains_key(&fd) {
            fd += 1;
        }
        self.next_fd.store(fd + 1, Ordering::Relaxed);

        // Add the file to the table
        self.files.insert(fd, file);
        fd // Return the new file descriptor number
    }

    /// Removes a file descriptor from the table.
    pub fn remove(&mut self, fd: usize) -> Option<Arc<File>> {
        self.files.remove(&fd)
    }

    /// Gets a reference to a File object from a descriptor ID.
    pub fn get(&self, fd: usize) -> Option<Arc<File>> {
        self.files.get(&fd).cloned() // Clone the Arc, not the File
    }
}
