use crate::fs::filesystem::{Error, FileDescriptor, FileSystem};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref FS: Mutex<BTreeMap<String, Box<dyn FileSystem + Send>>> =
        Mutex::new(BTreeMap::new());
}

pub fn init() {}

pub fn mount<T: FileSystem + Send + 'static>(mountpoint: &str, filesystem: T) {
    let mut fs = FS.lock();
    if mountpoint.contains("/") {
        todo!("mountpoint cant contain a /")
    }
    fs.insert(mountpoint.to_owned(), Box::new(filesystem));
}

pub fn open(path: &str) -> Result<FileDescriptor, Error> {
    let fs = FS.lock();
    let mount_point = get_mount_point(path);
    let path = remove_mount_point(path, mount_point);

    if let Some(device) = fs.get(mount_point) {
        Ok(FileDescriptor {
            file: device.open(&path)?,
            device: mount_point.to_owned(),
        })
    } else {
        Err(Error::DeviceDoesntExist)
    }
}

pub fn read(file: &FileDescriptor, buf: &mut [u8]) -> Result<(), Error> {
    let fs = FS.lock();

    if let Some(device) = fs.get(&file.device) {
        device.read(&file.file, buf)
    } else {
        Err(Error::DeviceDoesntExist)
    }
}

pub fn list_dir(path: &str) -> Result<Vec<FileDescriptor>, Error> {
    let fs = FS.lock();
    let mount_point = get_mount_point(path);
    let path = remove_mount_point(path, mount_point);

    if let Some(device) = fs.get(mount_point) {
        Ok(device
            .dir_entries(&path)?
            .iter()
            .map(|f| FileDescriptor {
                file: f.clone(),
                device: mount_point.to_owned(),
            })
            .collect())
    } else {
        Err(Error::DeviceDoesntExist)
    }
}

fn get_mount_point(path: &str) -> &str {
    let split: Vec<&str> = path.split("/").collect();
    let mount_point = *split.get(1).unwrap_or(&"");

    mount_point
}

fn remove_mount_point(path: &str, mount_point: &str) -> String {
    path.replace(&format!("/{mount_point}/"), "")
        .replace(&format!("/{mount_point}"), "")
}
