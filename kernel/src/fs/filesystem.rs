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
    fn read(&self, file: &File, buffer: &mut [u8]) -> Result<(), Error>;
}

#[derive(Default, Debug, Clone)]
pub struct File {
    pub name: String,
    pub path: String,
    pub r#type: String,
    pub size: u64,
}
