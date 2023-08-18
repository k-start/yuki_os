// Filesystem for storing STDIO for applications
use super::filesystem::{Error, File};
use alloc::{
    collections::{BTreeMap, VecDeque},
    format,
    string::ToString,
    vec::Vec,
};
use spin::Mutex;

pub struct StdioFs {
    fs: Mutex<BTreeMap<u32, Stdio>>,
}

impl super::filesystem::FileSystem for StdioFs {
    fn dir_entries(&self, dir: &str) -> Result<Vec<File>, Error> {
        let mut ret: Vec<File> = Vec::new();
        if dir == "" {
            for i in self.fs.lock().keys() {
                ret.push(File {
                    name: format!("{i}"),
                    path: format!("{i}"),
                    r#type: "dir".to_string(),
                    size: 0,
                    ptr: None,
                });
            }
            return Ok(ret);
        } else {
            let proc_id: u32 = dir.replace("/", "").parse().unwrap();
            if let Some(io) = self.fs.lock().get(&proc_id) {
                ret.push(File {
                    name: format!("stdin"),
                    path: format!("{proc_id}/stdin"),
                    r#type: "file".to_string(),
                    size: io.stdin.lock().len() as u64,
                    ptr: None,
                });
                ret.push(File {
                    name: format!("stdout"),
                    path: format!("{proc_id}/stdout"),
                    r#type: "file".to_string(),
                    size: io.stdout.lock().len() as u64,
                    ptr: None,
                });
            }
        }

        return Err(Error::DirDoesntExist);
    }

    fn open(&self, path: &str) -> Result<File, Error> {
        let split: Vec<&str> = path.split("/").collect();
        println!("1 - {split:?}");
        if split.len() != 2 {
            return Err(Error::FileDoesntExist);
        }

        let proc_id = split[0].parse::<u32>();
        println!("2 - {proc_id:?}");
        // if !split[1].eq("stdin") || !split[1].eq("stdout") {
        //     return Err(Error::FileDoesntExist);
        // }

        match proc_id {
            Ok(id) => {
                println!("3 - {id:?}");
                if let Some(_stdio) = self.fs.lock().get(&id) {
                    return Ok(File {
                        name: split[1].to_string(),
                        path: path.to_string(),
                        r#type: "file".to_string(),
                        size: 0, // fixme
                        ptr: None,
                    });
                }
                self.fs.lock().insert(id, Stdio::new());
                println!("4 - {id:?}");
                return Ok(File {
                    name: split[1].to_string(),
                    path: path.to_string(),
                    r#type: "file".to_string(),
                    size: 0, // fixme
                    ptr: None,
                });
            }
            Err(_) => return Err(Error::FileDoesntExist),
        };
    }

    fn read(&self, file: &File, buf: &mut [u8]) -> Result<(), Error> {
        let split: Vec<&str> = file.path.split("/").collect();
        if split.len() != 2 {
            return Err(Error::FileDoesntExist);
        }

        let proc_id = split[0].parse::<u32>();

        match proc_id {
            Ok(id) => {
                if let Some(stdio) = self.fs.lock().get(&id) {
                    match split[1] {
                        "stdin" => stdio.read_stdin(buf),
                        "stdout" => stdio.read_stdout(buf),
                        _ => return Err(Error::FileDoesntExist),
                    };
                }
                return Err(Error::FileDoesntExist);
            }
            Err(_) => return Err(Error::FileDoesntExist),
        };
    }

    fn write(&self, file: &File, buf: &[u8]) -> Result<(), Error> {
        let split: Vec<&str> = file.path.split("/").collect();
        if split.len() != 2 {
            return Err(Error::FileDoesntExist);
        }

        let proc_id = split[0].parse::<u32>();

        match proc_id {
            Ok(id) => {
                if let Some(stdio) = self.fs.lock().get(&id) {
                    match split[1] {
                        "stdin" => stdio.write_stdin(buf),
                        "stdout" => stdio.write_stdout(buf),
                        _ => return Err(Error::FileDoesntExist),
                    };
                }
                return Err(Error::FileDoesntExist);
            }
            Err(_) => return Err(Error::FileDoesntExist),
        };
    }
}

impl StdioFs {
    pub fn new() -> Self {
        StdioFs {
            fs: Mutex::new(BTreeMap::new()),
        }
    }
}

pub struct Stdio {
    stdout: Mutex<VecDeque<u8>>,
    stdin: Mutex<VecDeque<u8>>,
}

impl Stdio {
    pub fn new() -> Self {
        // fix me - mutexes
        Stdio {
            stdout: Mutex::new(VecDeque::new()),
            stdin: Mutex::new(VecDeque::new()),
        }
    }

    pub fn write_stdin(&self, buf: &[u8]) {
        for i in buf {
            self.stdin.lock().push_back(i.clone());
        }
    }

    pub fn write_stdout(&self, buf: &[u8]) {
        for i in buf {
            self.stdout.lock().push_back(i.clone());
        }
    }

    pub fn read_stdin(&self, buf: &mut [u8]) {
        for i in 0..buf.len() {
            buf[i] = self.stdin.lock().pop_front().unwrap_or(0);
        }
    }

    pub fn read_stdout(&self, buf: &mut [u8]) {
        for i in 0..buf.len() {
            buf[i] = self.stdout.lock().pop_front().unwrap_or(0);
        }
    }
}
