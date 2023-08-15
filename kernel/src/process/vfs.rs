use alloc::collections::VecDeque;

pub struct Vfs {
    stdout: VecDeque<u8>,
    stdin: VecDeque<u8>,
}

impl Vfs {
    pub fn new() -> Self {
        // fix me - mutexes
        Vfs {
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
