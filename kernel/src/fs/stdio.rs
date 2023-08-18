// Filesystem for storing STDIO for applications
use super::filesystem::{Error, File};
use alloc::{
    collections::{BTreeMap, VecDeque},
    format,
    string::ToString,
    vec::Vec,
};

pub struct StdioFs<'a> {
    fs: BTreeMap<i32, &'a Stdio>,
}

impl super::filesystem::FileSystem for StdioFs<'_> {
    fn dir_entries(&self, dir: &str) -> Result<Vec<File>, Error> {
        let mut ret: Vec<File> = Vec::new();
        if dir == "" {
            for i in self.fs.keys() {
                ret.push(File {
                    name: format!("pty{i}"),
                    path: format!("pty{i}"),
                    r#type: "dir".to_string(),
                    size: 0,
                    ptr: None,
                });
            }
            return Ok(ret);
        } else if dir.contains("pty") {
            let proc_id: i32 = dir.replace("pty", "").replace("/", "").parse().unwrap();
            if let Some(io) = self.fs.get(&proc_id) {
                ret.push(File {
                    name: format!("stdin"),
                    path: format!("pty{proc_id}/stdin"),
                    r#type: "file".to_string(),
                    size: io.stdin.len() as u64,
                    ptr: None,
                });
                ret.push(File {
                    name: format!("stdout"),
                    path: format!("pty{proc_id}/stdout"),
                    r#type: "file".to_string(),
                    size: io.stdout.len() as u64,
                    ptr: None,
                });
            }
        }

        return Err(Error::DirDoesntExist);
    }

    fn open(&self, path: &str) -> Result<super::filesystem::File, super::filesystem::Error> {
        todo!()
    }

    fn read(
        &self,
        file: &super::filesystem::File,
        buffer: &mut [u8],
    ) -> Result<(), super::filesystem::Error> {
        todo!()
    }
}

pub struct Stdio {
    stdout: VecDeque<u8>,
    stdin: VecDeque<u8>,
}

impl Stdio {
    pub fn new() -> Self {
        // fix me - mutexes
        Stdio {
            stdout: VecDeque::new(),
            stdin: VecDeque::new(),
        }
    }

    pub fn write_stdin(&mut self, buf: &[u8]) {
        for i in buf {
            self.stdin.push_back(i.clone());
        }
    }

    pub fn write_stdout(&mut self, buf: &[u8]) {
        for i in buf {
            self.stdout.push_back(i.clone());
        }
    }

    pub fn read_stdin(&mut self, buf: &mut [u8]) {
        for i in 0..buf.len() {
            buf[i] = self.stdin.pop_front().unwrap_or(0);
        }
    }

    pub fn read_stdout(&mut self, buf: &mut [u8]) {
        for i in 0..buf.len() {
            buf[i] = self.stdout.pop_front().unwrap_or(0);
        }
    }
}
