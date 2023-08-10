use crate::fs::filesystem::{Error, File};
use alloc::{borrow::ToOwned, vec::Vec};

pub struct InitRd<'a> {
    data: &'a [u8],
}

struct RdFile {
    filename: [u8; 32],
    size: usize,
}

impl super::filesystem::FileSystem for InitRd<'_> {
    fn dir_entries(&self, dir: &str) -> Result<Vec<File>, Error> {
        if dir != "" {
            return Err(Error::DirDoesntExist);
        }

        let mut ret: Vec<File> = Vec::new();

        let file_count = self.data[0];

        for i in 0..file_count {
            let rd_file: RdFile = unsafe {
                core::ptr::read(self.data.as_ptr().add((i as usize) * 40 + 1) as *const _)
            };

            let name = core::str::from_utf8(&rd_file.filename)
                .unwrap()
                .trim_matches(char::from(0));

            ret.push(File {
                name: name.to_owned(),
                path: name.to_owned(),
                r#type: "file".to_owned(),
                size: rd_file.size as u64,
            })
        }

        Ok(ret)
    }

    fn open(&self, path: &str) -> Result<File, Error> {
        todo!()
    }

    fn read(&self, file: &File, buffer: &mut [u8]) -> Result<(), Error> {
        todo!()
    }
}

impl InitRd<'_> {
    pub fn new(ptr: *const u8, len: usize) -> Self {
        let slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, len) };
        InitRd { data: slice }
    }
}
