use crate::vfs::{Filesystem, FsError, Inode, InodeKind, InodeRef};
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
};
use fatfs::{FileSystem, LossyOemCpConverter, NullTimeProvider, Read, Seek, SeekFrom, Write};
use spin::Mutex;

/// A wrapper for the `fatfs` filesystem to implement the VFS `Filesystem` trait.
/// It can be cloned to get multiple references to the same underlying filesystem.
#[derive(Clone)]
pub struct FatFs<IO: 'static + Read + Write + Seek + Send + Sync> {
    fs: Arc<Mutex<FileSystem<IO, NullTimeProvider, LossyOemCpConverter>>>,
}

impl<IO: Read + Write + Seek + Send + Sync> FatFs<IO> {
    /// Creates a new `FatFs` instance from a block device.
    pub fn new(device: IO) -> Result<Self, FsError> {
        let options = fatfs::FsOptions::new();
        let fs = FileSystem::new(device, options).map_err(|_e| FsError::IOError)?;
        Ok(Self {
            fs: Arc::new(Mutex::new(fs)),
        })
    }
}

impl<IO: Read + Write + Seek + Send + Sync> Filesystem for FatFs<IO> {
    fn root(&self) -> Result<InodeRef, FsError> {
        // The root inode represents the root directory of the filesystem
        let root_inode = Arc::new(FatInode::<IO> {
            fs: self.fs.clone(),
            path: "".to_string(),
            kind: InodeKind::Directory,
            size: 0, // FAT directories have a size of 0
        });
        Ok(root_inode)
    }
}

/// An Inode representation for a file or directory in a FAT filesystem.
/// It uses the path to identify the entry, as `fatfs` objects cannot be stored directly.
pub struct FatInode<IO: 'static + Read + Write + Seek + Send + Sync> {
    /// A reference to the underlying filesystem, shared among all inodes
    fs: Arc<Mutex<FileSystem<IO, NullTimeProvider, LossyOemCpConverter>>>,
    /// The full path from the root to this inode
    path: String,
    /// The kind of inode (file or directory)
    kind: InodeKind,
    /// The size of the file in bytes. Always 0 for directories.
    size: u64,
}

impl<IO: Read + Write + Seek + Send + Sync> Inode for FatInode<IO> {
    fn kind(&self) -> InodeKind {
        self.kind
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, FsError> {
        if self.kind() == InodeKind::Directory {
            return Err(FsError::IsADirectory);
        }

        // Lock the filesystem to perform I/O
        let fs = self.fs.lock();
        let mut file = fs
            .root_dir()
            .open_file(&self.path)
            .map_err(|_e| FsError::IOError)?;

        // Seek to the desired offset
        file.seek(SeekFrom::Start(offset))
            .map_err(|_e| FsError::IOError)?;

        // Read data into the buffer
        let bytes_read = file.read(buf).map_err(|_e| FsError::IOError)?;
        Ok(bytes_read)
    }

    fn write_at(&self, offset: u64, buf: &[u8]) -> Result<usize, FsError> {
        if self.kind() == InodeKind::Directory {
            return Err(FsError::IsADirectory);
        }

        let fs = self.fs.lock();
        let mut file = fs
            .root_dir()
            .open_file(&self.path)
            .map_err(|_e| FsError::IOError)?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|_e| FsError::IOError)?;

        let bytes_written = file.write(buf).map_err(|_e| FsError::IOError)?;
        Ok(bytes_written)
    }

    fn size(&self) -> u64 {
        todo!()
    }

    // fn list_entries(&self) -> Result<Vec<DirEntry>, FsError> {
    //     if self.kind() != InodeKind::Directory {
    //         return Err(FsError::NotADirectory);
    //     }

    //     let fs = self.fs.lock();
    //     let dir = fs
    //         .root_dir()
    //         .open_dir(&self.path)
    //         .map_err(|e| FsError::Implementation(format!("fatfs open_dir error: {:?}", e)))?;

    //     let mut entries = Vec::new();
    //     for entry_result in dir.iter() {
    //         let entry = entry_result
    //             .map_err(|e| FsError::Implementation(format!("fatfs iter error: {:?}", e)))?;

    //         // Skip '.' and '..' to avoid loops and confusion
    //         if entry.file_name() == "." || entry.file_name() == ".." {
    //             continue;
    //         }

    //         let entry_path = if self.path.is_empty() {
    //             entry.file_name()
    //         } else {
    //             format!("{}/{}", self.path, entry.file_name())
    //         };

    //         let kind = if entry.is_dir() {
    //             InodeKind::Directory
    //         } else {
    //             InodeKind::File
    //         };

    //         let inode: InodeRef = Arc::new(FatInode::<IO> {
    //             fs: self.fs.clone(),
    //             path: entry_path,
    //             kind,
    //             size: entry.len(),
    //         });

    //         entries.push(DirEntry {
    //             name: entry.file_name(),
    //             inode,
    //         });
    //     }
    //     Ok(entries)
    // }
}
