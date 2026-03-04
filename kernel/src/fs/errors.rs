#[derive(Debug, Clone, Copy)]
pub enum Error {
    FileDoesntExist,
    DirDoesntExist,
    DeviceDoesntExist,
    ReadError,
    PathSplitError,
    IoError,
}
