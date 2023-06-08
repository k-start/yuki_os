use alloc::{borrow::ToOwned, format, vec::Vec};
use fatfs::{FileSystem, LossyOemCpConverter, NullTimeProvider, Read, Seek, Write};

use crate::fs::filesystem::File;

pub struct FatFs<IO: Read + Write + Seek> {
    fs: FileSystem<IO, NullTimeProvider, LossyOemCpConverter>,
}

impl<IO: Read + Write + Seek> super::filesystem::FileSystem for FatFs<IO> {
    fn dir_entries(&self, dir: &str) -> Vec<File> {
        let mut ret: Vec<File> = Vec::new();
        let fs = &self.fs;
        let iter = match dir {
            "" => fs.root_dir().iter(),
            _ => fs.root_dir().open_dir(dir).unwrap().iter(),
        };

        for i in iter {
            if i.is_err() {
                continue;
            }
            let entry = i.unwrap();
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

        ret
    }

    fn open(&self, path: &str) -> Option<File> {
        let split: Vec<&str> = path.split("/").collect();
        let file_name = *split.last().unwrap();

        let fs = &self.fs;
        let mut dir = fs.root_dir();

        if split.len() > 1 {
            let path = path.replace(file_name, "");
            dir = dir.open_dir(&path).unwrap();
        }

        for file in dir.iter() {
            let file = file.unwrap();

            if file.file_name() == file_name {
                return Some(File {
                    name: file.file_name(),
                    path: path.to_owned(),
                    r#type: "file".to_owned(),
                    size: file.len(),
                });
            }
        }

        None
    }

    fn read(&self, file: &File, buffer: &mut [u8]) {
        let fs = &self.fs;
        let dir = fs.root_dir();

        let mut file = dir.open_file(&file.path).unwrap();
        file.read_exact(buffer).unwrap();
    }
}

impl<IO: Read + Write + Seek> FatFs<IO> {
    pub fn new(device: IO) -> Self {
        let fs = fatfs::FileSystem::new(device, fatfs::FsOptions::new()).unwrap();
        FatFs { fs }
    }
}
