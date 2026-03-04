use crate::fs::errors::Error;
use crate::fs::file::File;
use crate::fs::vnode::VNode;
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

static FS: Mutex<BTreeMap<String, Arc<dyn VNode>>> = Mutex::new(BTreeMap::new());

pub fn init() {}

pub fn mount(mountpoint: &str, filesystem: Arc<dyn VNode>) {
    let mut fs = FS.lock();
    if mountpoint.contains('/') {
        todo!("mountpoint cant contain a /")
    }
    fs.insert(mountpoint.to_owned(), filesystem);
}

pub fn open(path: &str) -> Result<Arc<Mutex<File>>, Error> {
    let fs = FS.lock();
    let mount_point = get_mount_point(path);
    let path = remove_mount_point(path, mount_point);

    if let Some(device) = fs.get(mount_point) {
        let vnode = if path.is_empty() {
            device.clone()
        } else {
            device.lookup(&path)?
        };
        let file = File::new(vnode, true, true);
        Ok(Arc::new(Mutex::new(file)))
    } else {
        Err(Error::DeviceDoesntExist)
    }
}

pub fn read(file: &Arc<Mutex<File>>, buf: &mut [u8]) -> Result<isize, Error> {
    file.lock().read(buf)
}

pub fn write(file: &Arc<Mutex<File>>, buf: &[u8]) -> Result<(), Error> {
    file.lock().write(buf)
}

pub fn ioctl(file: &Arc<Mutex<File>>, cmd: u32, args: usize) -> Result<(), Error> {
    file.lock().ioctl(cmd, args)
}

pub fn list_dir(path: &str) -> Result<Vec<String>, Error> {
    let fs = FS.lock();
    let mount_point = get_mount_point(path);
    let path = remove_mount_point(path, mount_point);

    if let Some(device) = fs.get(mount_point) {
        let vnode = if path.is_empty() {
            device.clone()
        } else {
            device.lookup(&path)?
        };
        vnode.dir_entries()
    } else {
        Err(Error::DeviceDoesntExist)
    }
}

fn get_mount_point(path: &str) -> &str {
    let split: Vec<&str> = path.split('/').collect();
    let mount_point = *split.get(1).unwrap_or(&"");

    mount_point
}

fn remove_mount_point(path: &str, mount_point: &str) -> String {
    path.replace(&format!("/{mount_point}/"), "")
        .replace(&format!("/{mount_point}"), "")
}
