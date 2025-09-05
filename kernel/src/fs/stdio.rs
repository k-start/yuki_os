// Filesystem for storing STDIO for applications
use crate::vfs::{Filesystem, FsError, Inode, InodeKind, InodeRef};
use alloc::{
    collections::{BTreeMap, VecDeque},
    format,
    string::String,
    string::ToString,
    sync::Arc,
    vec::Vec,
};
use spin::Mutex;

/// A virtual filesystem for process standard I/O streams (stdin, stdout)
///
/// This filesystem presents a directory for each process ID, containing
/// `stdin` and `stdout` files. It is designed to be managed by the kernel's
/// process management system, which should create and remove process entries.
#[derive(Clone)]
pub struct StdioFs {
    // Arc is needed to share the filesystem data with the inodes
    fs: Arc<Mutex<BTreeMap<u32, Arc<Stdio>>>>,
}

/// An inode in the StdioFs, representing the root, a process directory, or a stream
pub struct StdioInode {
    fs: Arc<Mutex<BTreeMap<u32, Arc<Stdio>>>>,
    kind: StdioInodeKind,
}

#[derive(Clone, Copy)]
enum StdioInodeKind {
    Root,
    ProcessDir(u32),
    Stream(u32, StreamKind),
}

#[derive(Clone, Copy)]
enum StreamKind {
    Stdin,
    Stdout,
}

impl Filesystem for StdioFs {
    fn root(&self) -> Result<InodeRef, FsError> {
        Ok(Arc::new(StdioInode {
            fs: self.fs.clone(),
            kind: StdioInodeKind::Root,
        }))
    }
}

impl Inode for StdioInode {
    fn kind(&self) -> InodeKind {
        match self.kind {
            StdioInodeKind::Root | StdioInodeKind::ProcessDir(_) => InodeKind::Directory,
            StdioInodeKind::Stream(_, _) => InodeKind::File,
        }
    }

    fn read_at(&self, _offset: u64, buf: &mut [u8]) -> Result<usize, FsError> {
        // offset is ignored for streams
        if let StdioInodeKind::Stream(pid, stream_kind) = self.kind {
            let fs_lock = self.fs.lock();
            let stdio = fs_lock.get(&pid).ok_or(FsError::NotFound)?;

            let bytes_read = match stream_kind {
                StreamKind::Stdin => stdio.read_stdin(buf),
                StreamKind::Stdout => stdio.read_stdout(buf),
            };
            Ok(bytes_read)
        } else {
            Err(FsError::IsADirectory)
        }
    }

    fn write_at(&self, _offset: u64, buf: &[u8]) -> Result<usize, FsError> {
        // offset is ignored for streams
        if let StdioInodeKind::Stream(pid, stream_kind) = self.kind {
            let fs_lock = self.fs.lock();
            let stdio = fs_lock.get(&pid).ok_or(FsError::NotFound)?;

            let bytes_written = match stream_kind {
                StreamKind::Stdin => stdio.write_stdin(buf),
                StreamKind::Stdout => stdio.write_stdout(buf),
            };
            Ok(bytes_written)
        } else {
            Err(FsError::IsADirectory)
        }
    }

    fn size(&self) -> u64 {
        todo!()
    }

    // fn list_entries(&self) -> Result<Vec<DirEntry>, FsError> {
    //     let mut entries = Vec::new();
    //     let fs_lock = self.fs.lock();

    //     match self.kind {
    //         StdioInodeKind::Root => {
    //             for pid in fs_lock.keys() {
    //                 let inode: InodeRef = Arc::new(StdioInode {
    //                     fs: self.fs.clone(),
    //                     kind: StdioInodeKind::ProcessDir(*pid),
    //                 });
    //                 entries.push(DirEntry {
    //                     name: pid.to_string(),
    //                     inode,
    //                 });
    //             }
    //         }
    //         StdioInodeKind::ProcessDir(pid) => {
    //             if !fs_lock.contains_key(&pid) {
    //                 return Err(FsError::EntryNotFound);
    //             }

    //             let stdin_inode: InodeRef = Arc::new(StdioInode {
    //                 fs: self.fs.clone(),
    //                 kind: StdioInodeKind::Stream(pid, StreamKind::Stdin),
    //             });
    //             entries.push(DirEntry {
    //                 name: "stdin".to_string(),
    //                 inode: stdin_inode,
    //             });

    //             let stdout_inode: InodeRef = Arc::new(StdioInode {
    //                 fs: self.fs.clone(),
    //                 kind: StdioInodeKind::Stream(pid, StreamKind::Stdout),
    //             });
    //             entries.push(DirEntry {
    //                 name: "stdout".to_string(),
    //                 inode: stdout_inode,
    //             });
    //         }
    //         StdioInodeKind::Stream(_, _) => return Err(FsError::NotADirectory),
    //     }
    //     Ok(entries)
    // }
}

impl Default for StdioFs {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioFs {
    /// Creates a new, empty StdioFs
    pub fn new() -> Self {
        StdioFs {
            fs: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Creates the stdio entries for a new process
    pub fn create_proc(&self, pid: u32) {
        self.fs.lock().insert(pid, Arc::new(Stdio::new()));
    }

    /// Removes the stdio entries for a finished process
    pub fn remove_proc(&self, pid: u32) {
        self.fs.lock().remove(&pid);
    }
}

/// Holds the stdin and stdout buffers for a single process
#[derive(Debug)]
pub struct Stdio {
    stdout: Mutex<VecDeque<u8>>,
    stdin: Mutex<VecDeque<u8>>,
}

impl Default for Stdio {
    fn default() -> Self {
        Self::new()
    }
}

impl Stdio {
    pub fn new() -> Self {
        Stdio {
            stdout: Mutex::new(VecDeque::new()),
            stdin: Mutex::new(VecDeque::new()),
        }
    }

    pub fn write_stdin(&self, buf: &[u8]) -> usize {
        let mut stdin_lock = self.stdin.lock();
        for &byte in buf {
            stdin_lock.push_back(byte);
        }
        buf.len()
    }

    pub fn write_stdout(&self, buf: &[u8]) -> usize {
        let mut stdout_lock = self.stdout.lock();
        for &byte in buf {
            stdout_lock.push_back(byte);
        }
        buf.len()
    }

    pub fn read_stdin(&self, buf: &mut [u8]) -> usize {
        let mut stdin_lock = self.stdin.lock();
        let mut len_read = 0;
        while len_read < buf.len() {
            if let Some(byte) = stdin_lock.pop_front() {
                buf[len_read] = byte;
                len_read += 1;
            } else {
                break; // No more data to read
            }
        }
        len_read
    }

    pub fn read_stdout(&self, buf: &mut [u8]) -> usize {
        let mut stdout_lock = self.stdout.lock();
        let mut len_read = 0;
        while len_read < buf.len() {
            if let Some(byte) = stdout_lock.pop_front() {
                buf[len_read] = byte;
                len_read += 1;
            } else {
                break; // No more data to read
            }
        }
        len_read
    }
}
