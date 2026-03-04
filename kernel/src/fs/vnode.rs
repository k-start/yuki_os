use crate::fs::errors::Error;
use alloc::{sync::Arc, vec::Vec};

/// Core interface for a Virtual Node (Inode)
pub trait VNode: Send + Sync {
    /// Read data from the vnode at the given offset
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<isize, Error>;

    /// Write data to the vnode at the given offset
    fn write(&self, offset: usize, buf: &[u8]) -> Result<(), Error>;

    /// Perform a device-specific control operation
    fn ioctl(&self, cmd: u32, arg: usize) -> Result<(), Error>;

    /// Return the size of the logical file
    fn size(&self) -> usize {
        0
    }

    /// Look up a child node by name (for directories)
    fn lookup(&self, _name: &str) -> Result<Arc<dyn VNode>, Error> {
        Err(Error::DirDoesntExist)
    }

    /// Read directory entries (if this is a directory)
    fn dir_entries(&self) -> Result<Vec<alloc::string::String>, Error> {
        Err(Error::DirDoesntExist)
    }
}
