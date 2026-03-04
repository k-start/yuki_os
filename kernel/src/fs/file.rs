use crate::fs::errors::Error;
use crate::fs::vnode::VNode;
use alloc::sync::Arc;
use spin::Mutex;

/// An open file description
pub struct File {
    pub vnode: Arc<dyn VNode>,
    /// The current byte offset for reading/writing
    pub offset: Mutex<usize>,
    pub readable: bool,
    pub writable: bool,
}

impl File {
    pub fn new(vnode: Arc<dyn VNode>, readable: bool, writable: bool) -> Self {
        File {
            vnode,
            offset: Mutex::new(0),
            readable,
            writable,
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<isize, Error> {
        if !self.readable {
            return Err(Error::ReadError);
        }
        let mut offset = self.offset.lock();
        let bytes_read = self.vnode.read(*offset, buf)?;
        if bytes_read > 0 {
            *offset += bytes_read as usize;
        }
        Ok(bytes_read)
    }

    pub fn write(&self, buf: &[u8]) -> Result<(), Error> {
        if !self.writable {
            return Err(Error::IoError); // Or a specific WriteError
        }
        let mut offset = self.offset.lock();
        self.vnode.write(*offset, buf)?;
        *offset += buf.len();
        Ok(())
    }

    pub fn seek(&self, pos: usize) -> Result<usize, Error> {
        let mut offset = self.offset.lock();
        *offset = pos;
        Ok(*offset)
    }

    pub fn ioctl(&self, cmd: u32, arg: usize) -> Result<(), Error> {
        self.vnode.ioctl(cmd, arg)
    }
}
