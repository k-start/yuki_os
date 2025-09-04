use crate::fs::{
    error::FsError,
    filesystem::Filesystem,
    inode::{DirEntry, Inode, InodeKind, InodeRef},
};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::mem;

// The header for a file in the initrd
// Using fixed-size integers is important for a consistent file format
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
struct RdFileHeader {
    filename: [u8; 32],
    size: u64,
    offset: u64,
}

/// A filesystem implementation for a simple, read-only initial ramdisk
#[derive(Clone)]
pub struct InitRdFs {
    // Arc allows sharing this data with all inodes
    data: Arc<&'static [u8]>,
}

impl InitRdFs {
    /// Creates a new InitRdFs from a memory region
    ///
    /// # Safety
    /// The caller must ensure that the pointer and length describe a valid
    /// and correctly formatted initrd that outlives the filesystem object
    /// (i.e., has a `'static` lifetime)
    pub unsafe fn new(ptr: *const u8, len: usize) -> Self {
        let slice: &'static [u8] = core::slice::from_raw_parts(ptr, len);
        Self {
            data: Arc::new(slice),
        }
    }

    // Helper to get file count
    fn file_count(&self) -> u8 {
        if self.data.is_empty() {
            0
        } else {
            self.data[0]
        }
    }

    // Helper to get a specific file header
    fn get_header(&self, index: u8) -> Option<RdFileHeader> {
        if index >= self.file_count() {
            return None;
        }
        let header_offset = 1 + (index as usize * mem::size_of::<RdFileHeader>());
        let header_end = header_offset + mem::size_of::<RdFileHeader>();
        if header_end > self.data.len() {
            return None;
        }

        let header_slice = &self.data[header_offset..header_end];
        // Safety: We've checked the bounds and RdFileHeader is packed
        Some(unsafe { core::ptr::read(header_slice.as_ptr() as *const _) })
    }

    // Helper to get the start of the data section
    fn data_section_start(&self) -> usize {
        1 + (self.file_count() as usize * mem::size_of::<RdFileHeader>())
    }
}

/// An Inode representation for a file or the root directory in an InitRd
pub struct InitRdInode {
    fs: InitRdFs,
    kind: InodeKind,
    // For files, this is the size
    size: u64,
    // For files, this is the absolute offset into the main data slice
    data_offset: u64,
}

impl Filesystem for InitRdFs {
    fn root(&self) -> Result<InodeRef, FsError> {
        let root_inode = Arc::new(InitRdInode {
            fs: self.clone(),
            kind: InodeKind::Directory,
            size: 0,
            data_offset: 0,
        });
        Ok(root_inode)
    }
}

impl Inode for InitRdInode {
    fn kind(&self) -> InodeKind {
        self.kind
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, FsError> {
        if self.kind() == InodeKind::Directory {
            return Err(FsError::IsADirectory);
        }

        // Ensure offset is within file bounds
        if offset >= self.size {
            return Ok(0); // Reading at or after EOF
        }

        let start = self.data_offset as usize + offset as usize;
        let end = (self.data_offset + self.size) as usize;
        let file_data = &self.fs.data[start..end];

        let bytes_to_copy = file_data.len().min(buf.len());
        buf[..bytes_to_copy].copy_from_slice(&file_data[..bytes_to_copy]);

        Ok(bytes_to_copy)
    }

    fn write_at(&self, _offset: u64, _buf: &[u8]) -> Result<usize, FsError> {
        Err(FsError::ReadOnly)
    }

    fn list_entries(&self) -> Result<Vec<DirEntry>, FsError> {
        if self.kind() != InodeKind::Directory {
            return Err(FsError::NotADirectory);
        }

        let mut entries = Vec::new();
        let file_count = self.fs.file_count();
        let data_start = self.fs.data_section_start();

        for i in 0..file_count {
            let header = self.fs.get_header(i).ok_or(FsError::InvalidPath)?;

            let name = core::str::from_utf8(&header.filename)
                .map_err(|_| FsError::InvalidPath)?
                .trim_matches('\0');

            let inode: InodeRef = Arc::new(InitRdInode {
                fs: self.fs.clone(),
                kind: InodeKind::File,
                size: header.size,
                data_offset: data_start as u64 + header.offset,
            });

            entries.push(DirEntry {
                name: name.to_string(),
                inode,
            });
        }

        Ok(entries)
    }

    fn size(&self) -> u64 {
        self.size
    }
}
