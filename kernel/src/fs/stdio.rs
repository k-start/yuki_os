// Filesystem for storing STDIO for applications
use crate::fs::errors::Error;
use crate::fs::vnode::VNode;
use alloc::{
    collections::{BTreeMap, VecDeque},
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::Mutex;

pub struct StdioFs {
    fs: Mutex<BTreeMap<u32, Arc<Stdio>>>,
}

impl VNode for StdioFs {
    fn dir_entries(&self) -> Result<Vec<String>, Error> {
        let mut ret = Vec::new();
        for key in self.fs.lock().keys() {
            ret.push(format!("{key}"));
        }
        Ok(ret)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, Error> {
        let split: Vec<&str> = name.split('/').collect();
        if split.is_empty() {
            return Err(Error::FileDoesntExist);
        }

        let proc_id = split[0]
            .parse::<u32>()
            .map_err(|_| Error::FileDoesntExist)?;

        let mut fs = self.fs.lock();
        let stdio = fs
            .entry(proc_id)
            .or_insert_with(|| Arc::new(Stdio::new()))
            .clone();

        if split.len() == 1 {
            Ok(Arc::new(ProcessStdioFs { stdio }))
        } else if split.len() == 2 {
            match split[1] {
                "stdin" => Ok(Arc::new(StdinVNode { stdio })),
                "stdout" => Ok(Arc::new(StdoutVNode { stdio })),
                _ => Err(Error::FileDoesntExist),
            }
        } else {
            Err(Error::FileDoesntExist)
        }
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

pub struct ProcessStdioFs {
    stdio: Arc<Stdio>,
}

impl VNode for ProcessStdioFs {
    fn dir_entries(&self) -> Result<Vec<String>, Error> {
        Ok(alloc::vec!["stdin".to_string(), "stdout".to_string()])
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, Error> {
        match name {
            "stdin" => Ok(Arc::new(StdinVNode {
                stdio: self.stdio.clone(),
            })),
            "stdout" => Ok(Arc::new(StdoutVNode {
                stdio: self.stdio.clone(),
            })),
            _ => Err(Error::FileDoesntExist),
        }
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

struct StdinVNode {
    stdio: Arc<Stdio>,
}

impl VNode for StdinVNode {
    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<isize, Error> {
        Ok(self.stdio.read_stdin(buf))
    }
    fn write(&self, _offset: usize, buf: &[u8]) -> Result<(), Error> {
        self.stdio.write_stdin(buf);
        Ok(())
    }
    fn ioctl(&self, _cmd: u32, _arg: usize) -> Result<(), Error> {
        Err(Error::IoError)
    }
}

struct StdoutVNode {
    stdio: Arc<Stdio>,
}

impl VNode for StdoutVNode {
    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<isize, Error> {
        Ok(self.stdio.read_stdout(buf))
    }
    fn write(&self, _offset: usize, buf: &[u8]) -> Result<(), Error> {
        self.stdio.write_stdout(buf);
        Ok(())
    }
    fn ioctl(&self, _cmd: u32, _arg: usize) -> Result<(), Error> {
        Err(Error::IoError)
    }
}

impl Default for StdioFs {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioFs {
    pub fn new() -> Self {
        StdioFs {
            fs: Mutex::new(BTreeMap::new()),
        }
    }
}

#[derive(Debug)]
pub struct Stdio {
    stdout: Mutex<VecDeque<u8>>,
    stdin: Mutex<VecDeque<u8>>,
}

impl Default for Stdio {
    fn default() -> Self {
        Self::new()
    }
}

impl Stdio {
    pub fn new() -> Self {
        Stdio {
            stdout: Mutex::new(VecDeque::new()),
            stdin: Mutex::new(VecDeque::new()),
        }
    }

    pub fn write_stdin(&self, buf: &[u8]) {
        for i in buf {
            self.stdin.lock().push_back(*i);
        }
    }

    pub fn write_stdout(&self, buf: &[u8]) {
        for i in buf {
            self.stdout.lock().push_back(*i);
        }
    }

    pub fn read_stdin(&self, buf: &mut [u8]) -> isize {
        let mut len_read = 0;
        for item in buf {
            *item = {
                let data = self.stdin.lock().pop_front();
                match data {
                    Some(x) => {
                        len_read += 1;
                        x
                    }
                    None => break,
                }
            };
        }
        len_read
    }

    pub fn read_stdout(&self, buf: &mut [u8]) -> isize {
        let mut len_read = 0;
        for item in buf {
            *item = {
                let data = self.stdout.lock().pop_front();
                match data {
                    Some(x) => {
                        len_read += 1;
                        x
                    }
                    None => break,
                }
            };
        }
        len_read
    }
}
