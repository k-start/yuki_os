// Filesystem for storing STDIO for applications
use super::filesystem::{Error, File};
use alloc::{
    collections::{BTreeMap, VecDeque},
    format,
    string::{String, ToString},
    vec::Vec,
};
use spin::Mutex;

pub struct DevFs {
    fs: Mutex<BTreeMap<String, Device>>,
}

impl super::filesystem::FileSystem for DevFs {
    fn dir_entries(&self, dir: &str) -> Result<Vec<File>, Error> {
        let mut ret: Vec<File> = Vec::new();
        if dir.is_empty() {
            for i in self.fs.lock().keys() {
                ret.push(File {
                    name: format!("{i}"),
                    path: format!("{i}"),
                    r#type: "file".to_string(),
                    size: 0,
                    ptr: None,
                    offset: 0,
                });
            }
            return Ok(ret);
        }

        Err(Error::DirDoesntExist)
    }

    fn open(&self, path: &str) -> Result<File, Error> {
        if let Some(_device) = self.fs.lock().get(path) {
            return Ok(File {
                name: path.to_string(),
                path: path.to_string(),
                r#type: "file".to_string(),
                size: 0, // fixme
                ptr: None,
                offset: 0,
            });
        }

        self.fs.lock().insert(path.to_string(), Device::new());
        Ok(File {
            name: path.to_string(),
            path: path.to_string(),
            r#type: "file".to_string(),
            size: 0, // fixme
            ptr: None,
            offset: 0,
        })
    }

    fn read(&self, file: &File, buf: &mut [u8]) -> Result<isize, Error> {
        if let Some(device) = self.fs.lock().get(&file.path) {
            let len = device.read(buf);
            return Ok(len);
        }

        Err(Error::FileDoesntExist)
    }

    fn write(&self, file: &File, buf: &[u8]) -> Result<(), Error> {
        if let Some(device) = self.fs.lock().get(&file.path) {
            device.write(buf);
            return Ok(());
        }

        Err(Error::FileDoesntExist)
    }

    fn ioctl(&self, _file: &File, _cmd: u32, _arg: usize) -> Result<(), Error> {
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

impl Device {
    pub fn new() -> Self {
        // fix me - mutexes
        Device {
            data: Mutex::new(VecDeque::new()),
        }
    }

    pub fn write(&self, buf: &[u8]) {
        for i in buf {
            self.data.lock().push_back(*i);
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> isize {
        let mut len_read = 0;
        let mut data = self.data.lock();
        for item in buf {
            let byte = data.pop_front();
            match byte {
                Some(x) => {
                    len_read = len_read + 1;
                    *item = x;
                }
                None => {}
            };
        }
        len_read
    }
}
