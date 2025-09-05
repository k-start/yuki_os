// Publicly export the main traits and structs
pub use self::error::FsError;
pub use self::file::{File, FileDescriptorTable};
pub use self::inode::{DirEntry, Inode, InodeKind, InodeRef};
pub use self::mount::{mount, open, Filesystem, FilesystemRef, MountTable};

mod error;
mod file;
mod inode;
mod mount;
