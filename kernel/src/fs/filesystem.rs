use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, Copy)]
pub enum Error {
    FileDoesntExist,
    DirDoesntExist,
    DeviceDoesntExist,
    ReadError,
    PathSplitError,
    IoError,
}

pub trait FileSystem {
    fn dir_entries(&self, dir: &str) -> Result<Vec<File>, Error>;
    fn open(&self, path: &str) -> Result<File, Error>;
    fn read(&self, file: &File, buf: &mut [u8]) -> Result<isize, Error>;
    fn write(&self, file: &File, buf: &[u8]) -> Result<(), Error>;
    fn ioctl(&self, file: &File, cmd: u32, arg: usize) -> Result<(), Error>;
}

#[derive(Default, Debug, Clone)]
pub struct File {
    pub name: String,
    pub path: String,
    pub r#type: String,
    pub size: u64,
    pub ptr: Option<u64>,
}

#[derive(Default, Debug, Clone)]
pub struct FileDescriptor {
    pub file: File,
    pub device: String,
}
