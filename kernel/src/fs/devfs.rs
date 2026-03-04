// Filesystem for storing STDIO for applications
use crate::fs::errors::Error;
use crate::fs::vnode::VNode;
use alloc::sync::Arc;
use alloc::{
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
    vec::Vec,
};
use spin::Mutex;

pub struct DevFs {
    fs: Mutex<BTreeMap<String, Arc<Device>>>,
}

impl VNode for DevFs {
    fn dir_entries(&self) -> Result<Vec<String>, Error> {
        let mut ret = Vec::new();
        for key in self.fs.lock().keys() {
            ret.push(key.clone());
        }
        Ok(ret)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, Error> {
        let mut fs = self.fs.lock();
        if let Some(device) = fs.get(name) {
            return Ok(device.clone() as Arc<dyn VNode>);
        }

        let new_device = Arc::new(Device::new());
        fs.insert(name.to_string(), new_device.clone());
        Ok(new_device as Arc<dyn VNode>)
    }

    fn read(&self, _offset: usize, _buf: &mut [u8]) -> Result<isize, Error> {
        Err(Error::ReadError) // Cannot read the directory itself as a file
    }

    fn write(&self, _offset: usize, _buf: &[u8]) -> Result<(), Error> {
        Err(Error::IoError)
    }

    fn ioctl(&self, _cmd: u32, _arg: usize) -> Result<(), Error> {
        todo!()
    }
}

impl Default for DevFs {
    fn default() -> Self {
        Self::new()
    }
}

impl DevFs {
    pub fn new() -> Self {
        DevFs {
            fs: Mutex::new(BTreeMap::new()),
        }
    }
}

#[derive(Debug)]
pub struct Device {
    data: Mutex<VecDeque<u8>>,
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

impl VNode for Device {
    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<isize, Error> {
        let mut len_read = 0;
        let mut data = self.data.lock();
        for item in buf {
            match data.pop_front() {
                Some(x) => {
                    len_read += 1;
                    *item = x;
                }
                None => break,
            }
        }
        Ok(len_read as isize)
    }

    fn write(&self, _offset: usize, buf: &[u8]) -> Result<(), Error> {
        let mut data = self.data.lock();
        for i in buf {
            data.push_back(*i);
        }
        Ok(())
    }

    fn ioctl(&self, _cmd: u32, _arg: usize) -> Result<(), Error> {
        todo!()
    }
}

impl Device {
    pub fn new() -> Self {
        // fix me - mutexes
        Device {
            data: Mutex::new(VecDeque::new()),
        }
    }
}
