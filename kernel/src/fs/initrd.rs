use crate::fs::errors::Error;
use crate::fs::vnode::VNode;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub struct InitRd {
    data: &'static [u8],
}

struct RdFile {
    filename: [u8; 32],
    size: usize,
    offset: usize,
}

impl VNode for InitRd {
    fn dir_entries(&self) -> Result<Vec<String>, Error> {
        let mut ret = Vec::new();
        let file_count = self.data[0];

        for i in 0..file_count {
            let rd_file: RdFile = unsafe {
                core::ptr::read(
                    self.data
                        .as_ptr()
                        .add((i as usize) * core::mem::size_of::<RdFile>() + 1)
                        as *const _,
                )
            };

            let name = core::str::from_utf8(&rd_file.filename)
                .unwrap()
                .trim_matches(char::from(0));
            ret.push(name.to_owned());
        }

        Ok(ret)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, Error> {
        let file_count = self.data[0];

        for i in 0..file_count {
            let rd_file: RdFile = unsafe {
                core::ptr::read(
                    self.data
                        .as_ptr()
                        .add((i as usize) * core::mem::size_of::<RdFile>() + 1)
                        as *const _,
                )
            };

            let file_name = core::str::from_utf8(&rd_file.filename)
                .unwrap()
                .trim_matches(char::from(0));

            if file_name == name {
                let offset = rd_file.offset as usize
                    + file_count as usize * core::mem::size_of::<RdFile>()
                    + 1;

                return Ok(Arc::new(InitRdNode {
                    data: self.data,
                    start: offset,
                    size: rd_file.size,
                }));
            }
        }

        Err(Error::FileDoesntExist)
    }

    fn read(&self, _offset: usize, _buf: &mut [u8]) -> Result<isize, Error> {
        Err(Error::ReadError)
    }

    fn write(&self, _offset: usize, _buf: &[u8]) -> Result<(), Error> {
        Err(Error::IoError)
    }

    fn ioctl(&self, _cmd: u32, _arg: usize) -> Result<(), Error> {
        Err(Error::IoError)
    }
}

pub struct InitRdNode {
    data: &'static [u8],
    start: usize,
    size: usize,
}

impl VNode for InitRdNode {
    fn size(&self) -> usize {
        self.size
    }

    fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<isize, Error> {
        if offset >= self.size {
            return Ok(0);
        }
        let available = self.size - offset;
        let to_read = core::cmp::min(buffer.len(), available);
        buffer[..to_read]
            .copy_from_slice(&self.data[(self.start + offset)..(self.start + offset + to_read)]);
        Ok(to_read as isize)
    }

    fn write(&self, _offset: usize, _buf: &[u8]) -> Result<(), Error> {
        panic!("Can't write to initrd")
    }

    fn ioctl(&self, _cmd: u32, _arg: usize) -> Result<(), Error> {
        Err(Error::IoError)
    }
}

impl InitRd {
    /// Initialize a new OffsetPageTable.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it creates a slice from the pointer
    /// provided
    pub unsafe fn new(ptr: *const u8, len: usize) -> Self {
        let slice: &'static [u8] = core::slice::from_raw_parts(ptr, len);
        InitRd { data: slice }
    }
}
