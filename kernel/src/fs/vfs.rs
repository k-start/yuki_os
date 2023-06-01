use alloc::boxed::Box;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::filesystem::FileSystem;

lazy_static! {
    static ref FS: Mutex<Vec<Box<dyn FileSystem + Send>>> = Mutex::new(Vec::new());
}

pub fn init() {}

pub fn mount<T: FileSystem + Send + 'static>(filesystem: T) {
    let mut fs = FS.lock();
    fs.push(Box::new(filesystem));
}

pub fn open(path: &str) {
    let fs = FS.lock();
    let split: Vec<&str> = path.split(":/").collect();

    if split.len() != 2 {
        return;
    }

    let device = split[0].chars().next().unwrap() as u8 - b'a';

    if device as usize >= fs.len() {
        return;
    }

    fs[device as usize].open(split[1]);
}
