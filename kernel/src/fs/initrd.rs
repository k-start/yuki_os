use crate::fs::filesystem::{Error, File};
use alloc::{borrow::ToOwned, vec::Vec};

pub struct InitRd<'a> {
    data: &'a [u8],
}

struct RdFile {
    filename: [u8; 32],
    size: usize,
    offset: usize,
}

impl super::filesystem::FileSystem for InitRd<'_> {
    fn dir_entries(&self, dir: &str) -> Result<Vec<File>, Error> {
        if !dir.is_empty() {
            return Err(Error::DirDoesntExist);
        }

        let mut ret: Vec<File> = Vec::new();

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

            ret.push(File {
                name: name.to_owned(),
                path: name.to_owned(),
                r#type: "file".to_owned(),
                size: rd_file.size as u64,
                ptr: Some(unsafe {
                    self.data.as_ptr().add(
                        rd_file.offset as usize
                            + file_count as usize * core::mem::size_of::<RdFile>()
                            + 1,
                    ) as u64
                }),
            })
        }

        Ok(ret)
    }

    fn open(&self, path: &str) -> Result<File, Error> {
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
            if name == path {
                return Ok(File {
                    name: name.to_owned(),
                    path: name.to_owned(),
                    r#type: "file".to_owned(),
                    size: rd_file.size as u64,
                    ptr: Some(unsafe {
                        self.data.as_ptr().add(
                            rd_file.offset as usize
                                + file_count as usize * core::mem::size_of::<RdFile>()
                                + 1,
                        ) as u64
                    }),
                });
            }
        }

        Err(Error::FileDoesntExist)
    }

    fn read(&self, file: &File, buffer: &mut [u8]) -> Result<(), Error> {
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

            if name == file.name {
                let offset = rd_file.offset as usize
                    + file_count as usize * core::mem::size_of::<RdFile>()
                    + 1;

                buffer[..(offset + rd_file.size - offset)]
                    .copy_from_slice(&self.data[offset..(offset + rd_file.size)]);
            }
        }
        Ok(())
    }

    fn write(&self, _file: &File, _buf: &[u8]) -> Result<(), Error> {
        panic!("Can't write to initrd")
    }
}

impl InitRd<'_> {
    /// Initialize a new OffsetPageTable.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it creates a slice from the pointer
    /// provided
    pub unsafe fn new(ptr: *const u8, len: usize) -> Self {
        let slice: &[u8] = core::slice::from_raw_parts(ptr, len);
        InitRd { data: slice }
    }
}
