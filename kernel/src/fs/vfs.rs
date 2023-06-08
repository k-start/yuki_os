use alloc::boxed::Box;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::filesystem::FileSystem;

use super::filesystem::File;

lazy_static! {
    static ref FS: Mutex<Vec<Box<dyn FileSystem + Send>>> = Mutex::new(Vec::new());
}

pub fn init() {}

pub fn mount<T: FileSystem + Send + 'static>(filesystem: T) {
    let mut fs = FS.lock();
    fs.push(Box::new(filesystem));
}

pub fn open(path: &str) -> Option<File> {
    let fs = FS.lock();
    let (device, path) = get_device(path);

    if device as usize >= fs.len() {
        return None;
    }

    fs[device as usize].open(path)
}

pub fn read(file: &File, buf: &mut [u8]) {
    let fs = FS.lock();

    fs[0].read(file, buf);
}

pub fn list_dir(path: &str) -> Option<Vec<File>> {
    let fs = FS.lock();
    let (device, path) = get_device(path);

    if device as usize >= fs.len() {
        return None;
    }

    Some(fs[device as usize].dir_entries(path))
}

fn get_device(path: &str) -> (u8, &str) {
    let split: Vec<&str> = path.split(":/").collect();

    if split.len() != 2 {
        return (100, "");
    }

    (split[0].chars().next().unwrap() as u8 - b'a', split[1])
}
