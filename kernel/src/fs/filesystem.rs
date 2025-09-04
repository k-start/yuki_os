use alloc::sync::Arc;

use crate::fs::{error::FsError, inode::InodeRef};

pub trait Filesystem: Send + Sync {
    /// Returns the root inode for this filesystem.
    fn root(&self) -> Result<InodeRef, FsError>;

    // You could add other filesystem-wide operations here, like `sync`.
    // fn sync(&self) -> Result<(), FsError>;
}

pub type FilesystemRef = Arc<dyn Filesystem>;
