use crate::fs::filesystem::{Error, File};
use alloc::{borrow::ToOwned, format, vec::Vec};
use fatfs::{FileSystem, LossyOemCpConverter, NullTimeProvider, Read, Seek, Write};

pub struct FatFs<IO: Read + Write + Seek> {
    fs: FileSystem<IO, NullTimeProvider, LossyOemCpConverter>,
}

impl<IO: Read + Write + Seek> super::filesystem::FileSystem for FatFs<IO> {
    fn dir_entries(&self, dir: &str) -> Result<Vec<File>, Error> {
        let mut ret: Vec<File> = Vec::new();
        let fs = &self.fs;
        let dir_entry = match dir {
            "" => Ok(fs.root_dir()),
            _ => fs.root_dir().open_dir(dir),
        };

        let iter = match dir_entry {
            Ok(x) => x.iter(),
            Err(_) => return Err(Error::DirDoesntExist),
        };

        for i in iter {
            if i.is_err() {
                continue;
            }
            let entry = i.map_err(|_| Error::IoError)?;
            ret.push(File {
                name: entry.file_name(),
                path: format!("{}/{}", dir, entry.file_name()),
                r#type: match entry.is_file() {
                    true => "file".to_owned(),
                    false => "dir".to_owned(),
                },
                size: entry.len(),
            });
        }

        Ok(ret)
    }

    fn open(&self, path: &str) -> Result<File, Error> {
        let split: Vec<&str> = path.split("/").collect();
        let file_name = match split.last() {
            Some(x) => *x,
            None => return Err(Error::PathSplitError),
        };

        let fs = &self.fs;
        let mut dir = fs.root_dir();

        if split.len() > 1 {
            let path = path.replace(file_name, "");
            dir = dir.open_dir(&path).map_err(|_| Error::FileDoesntExist)?;
        }

        for file in dir.iter() {
            let file = file.map_err(|_| Error::IoError)?;

            if file.file_name() == file_name {
                return Ok(File {
                    name: file.file_name(),
                    path: path.to_owned(),
                    r#type: "file".to_owned(),
                    size: file.len(),
                });
            }
        }

        Err(Error::FileDoesntExist)
    }

    fn read(&self, file: &File, buffer: &mut [u8]) -> Result<(), Error> {
        let fs = &self.fs;
        let dir = fs.root_dir();

        let mut file = dir
            .open_file(&file.path)
            .map_err(|_| Error::FileDoesntExist)?;

        file.read_exact(buffer).map_err(|_| Error::ReadError)?;

        Ok(())
    }
}

impl<IO: Read + Write + Seek> FatFs<IO> {
    pub fn new(device: IO) -> Self {
        let fs = fatfs::FileSystem::new(device, fatfs::FsOptions::new()).unwrap();
        FatFs { fs }
    }
}
