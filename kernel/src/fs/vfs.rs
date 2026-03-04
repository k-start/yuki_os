use crate::fs::errors::Error;
use crate::fs::file::File;
use crate::fs::vnode::VNode;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

pub struct Mount {
    pub mountpoint: String,
    pub root: Arc<dyn VNode>,
}

static FS: Mutex<Vec<Mount>> = Mutex::new(Vec::new());

pub fn init() {}

/// Canonicalizes a path by resolving `.` and `..` and removing redundant slashes
/// Returns a path string that always starts with `/` unless the path is truly empty/invalid
fn canonicalize_path(path: &str) -> String {
    let mut components = Vec::new();

    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        } else if part == ".." {
            components.pop();
        } else {
            components.push(part);
        }
    }

    let mut result = String::from("/");
    result.push_str(&components.join("/"));
    result
}

pub fn mount(mountpoint: &str, filesystem: Arc<dyn VNode>) {
    let mut fs = FS.lock();
    let canonical = canonicalize_path(mountpoint);

    // Check if already mounted
    if fs.iter().any(|m| m.mountpoint == canonical) {
        // fixme: Return error here if we want
        // For now we don't really care
    }

    fs.push(Mount {
        mountpoint: canonical,
        root: filesystem,
    });

    // Sort by mountpoint length descending, so longest prefixes match first
    // (e.g., /mnt/usb/ matches before /mnt/)
    fs.sort_by(|a, b| b.mountpoint.len().cmp(&a.mountpoint.len()));
}

fn resolve_path(path: &str) -> (Option<Arc<dyn VNode>>, String) {
    let canonical = canonicalize_path(path);
    let mut canonical_slash = canonical.clone();
    if !canonical_slash.ends_with('/') {
        canonical_slash.push('/');
    }

    let fs = FS.lock();
    for mount in fs.iter() {
        let mut mountpoint_slash = mount.mountpoint.clone();
        if !mountpoint_slash.ends_with('/') {
            mountpoint_slash.push('/');
        }

        if canonical_slash.starts_with(&mountpoint_slash) {
            let remaining = &canonical[mount.mountpoint.len()..];
            let remaining = remaining.trim_start_matches('/');
            return (Some(mount.root.clone()), String::from(remaining));
        }
    }

    (None, String::new())
}

pub fn open(path: &str) -> Result<Arc<Mutex<File>>, Error> {
    let (vnode, remaining) = resolve_path(path);

    if let Some(device) = vnode {
        let final_vnode = if remaining.is_empty() {
            device.clone()
        } else {
            device.lookup(&remaining)?
        };
        let file = File::new(final_vnode, true, true);
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
    let (vnode, remaining) = resolve_path(path);

    if let Some(device) = vnode {
        let final_vnode = if remaining.is_empty() {
            device.clone()
        } else {
            device.lookup(&remaining)?
        };
        final_vnode.dir_entries()
    } else {
        Err(Error::DeviceDoesntExist)
    }
}
