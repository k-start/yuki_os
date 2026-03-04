use crate::fs::errors::Error;
use crate::fs::vnode::VNode;
use alloc::{borrow::ToOwned, string::String, sync::Arc, vec::Vec};
use fatfs::{FileSystem, LossyOemCpConverter, NullTimeProvider, Read, Seek, SeekFrom, Write};
use spin::Mutex;

pub struct FatFs<IO: Read + Write + Seek> {
    fs: Arc<Mutex<FileSystem<IO, NullTimeProvider, LossyOemCpConverter>>>,
}

impl<IO: Read + Write + Seek + Send + Sync + 'static> VNode for FatFs<IO> {
    fn dir_entries(&self) -> Result<Vec<String>, Error> {
        let mut ret = Vec::new();
        let fs_lock = self.fs.lock();
        let dir = fs_lock.root_dir();

        for entry in dir.iter() {
            if let Ok(e) = entry {
                ret.push(e.file_name());
            }
        }

        Ok(ret)
    }

    fn lookup(&self, path: &str) -> Result<Arc<dyn VNode>, Error> {
        let fs_lock = self.fs.lock();
        let root = fs_lock.root_dir();

        // Verify it exists
        root.open_file(path).map_err(|_| Error::FileDoesntExist)?;

        Ok(Arc::new(FatFileNode {
            fs: self.fs.clone(),
            path: path.to_owned(),
        }))
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

pub struct FatFileNode<IO: Read + Write + Seek> {
    fs: Arc<Mutex<FileSystem<IO, NullTimeProvider, LossyOemCpConverter>>>,
    path: String,
}

impl<IO: Read + Write + Seek + Send + Sync + 'static> VNode for FatFileNode<IO> {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<isize, Error> {
        let fs_lock = self.fs.lock();
        let root = fs_lock.root_dir();
        let mut file = root
            .open_file(&self.path)
            .map_err(|_| Error::FileDoesntExist)?;

        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|_| Error::ReadError)?;
        let bytes_read = file.read(buf).map_err(|_| Error::ReadError)?;

        Ok(bytes_read as isize)
    }

    fn write(&self, offset: usize, buf: &[u8]) -> Result<(), Error> {
        let fs_lock = self.fs.lock();
        let root = fs_lock.root_dir();
        let mut file = root
            .open_file(&self.path)
            .map_err(|_| Error::FileDoesntExist)?;

        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|_| Error::IoError)?;
        file.write_all(buf).map_err(|_| Error::IoError)?;

        Ok(())
    }

    fn ioctl(&self, _cmd: u32, _arg: usize) -> Result<(), Error> {
        Err(Error::IoError)
    }
}

impl<IO: Read + Write + Seek> FatFs<IO> {
    pub fn new(device: IO) -> Self {
        let fs = fatfs::FileSystem::new(device, fatfs::FsOptions::new()).unwrap();
        FatFs {
            fs: Arc::new(Mutex::new(fs)),
        }
    }
}
