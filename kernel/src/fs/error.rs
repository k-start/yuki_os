#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// No such file or directory (ENOENT)
    NotFound,
    /// Permission denied (EACCES)
    PermissionDenied,
    /// The file already exists (EEXIST)
    AlreadyExists,
    /// Not a directory (ENOTDIR)
    NotADirectory,
    /// Is a directory (EISDIR)
    IsADirectory,
    /// The filesystem is full (ENOSPC)
    NoSpace,
    /// The file is too large (EFBIG)
    FileTooLarge,
    /// Reached the end of the file (EOF)
    EndOfFile,
    /// A generic I/O error from a lower-level driver (EIO)
    IOError,
    /// The path or name is invalid (EINVAL)
    InvalidPath,
    ReadOnly,
    MountPointNotFound,
}
