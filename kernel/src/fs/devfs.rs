use crate::fs::framebuffer::FramebufferDevice;
use crate::vfs::{DirEntry, Filesystem, FsError, Inode, InodeKind, InodeRef};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use bootloader_api::info::FrameBuffer;

// DevFs is the filesystem for virtual device files
#[derive(Clone)]
pub struct DevFs {
    // A map from a device name (e.g., "fb0") to its Inode
    // Arc allows sharing this map with the root inode
    devices: Arc<BTreeMap<String, InodeRef>>,
}

impl DevFs {
    pub fn new(framebuffer: FrameBuffer) -> Self {
        let mut devices: BTreeMap<String, InodeRef> = BTreeMap::new();
        // Create and add your devices when the devfs is initialized
        devices.insert(
            "fb0".to_string(),
            Arc::new(FramebufferDevice::new(framebuffer)),
        );
        Self {
            devices: Arc::new(devices),
        }
    }
}

impl Filesystem for DevFs {
    fn root(&self) -> Result<InodeRef, FsError> {
        let root_inode = Arc::new(DevFsRootInode {
            devices: self.devices.clone(),
        });
        Ok(root_inode)
    }
}

// The root inode for the device filesystem
struct DevFsRootInode {
    devices: Arc<BTreeMap<String, InodeRef>>,
}

impl Inode for DevFsRootInode {
    fn kind(&self) -> InodeKind {
        InodeKind::Directory
    }

    fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, FsError> {
        Err(FsError::IsADirectory)
    }

    fn write_at(&self, _offset: u64, _buf: &[u8]) -> Result<usize, FsError> {
        Err(FsError::IsADirectory)
    }

    fn size(&self) -> u64 {
        todo!()
    }

    fn list_entries(&self) -> Result<Vec<DirEntry>, FsError> {
        let entries = self
            .devices
            .iter()
            .map(|(name, inode)| DirEntry {
                name: name.clone(),
                inode: inode.clone(),
            })
            .collect();
        Ok(entries)
    }
}
