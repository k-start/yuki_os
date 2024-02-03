// Filesystem for storing the framebuffer for applications to draw to the screen
use super::filesystem::{Error, File};
use bootloader_api::info::FrameBuffer;

pub struct FrameBufferFs<'a> {
    framebuffer: &'a FrameBuffer,
}

impl FrameBufferFs<'static> {
    pub const fn new(framebuffer: &'static FrameBuffer) -> FrameBufferFs<'static> {
        FrameBufferFs { framebuffer }
    }
}

impl super::filesystem::FileSystem for FrameBufferFs<'static> {
    fn dir_entries(&self, dir: &str) -> Result<alloc::vec::Vec<File>, Error> {
        todo!()
    }

    fn open(&self, path: &str) -> Result<File, Error> {
        todo!()
    }

    fn read(&self, file: &File, buf: &mut [u8]) -> Result<(), Error> {
        todo!()
    }

    fn write(&self, file: &File, buf: &[u8]) -> Result<(), Error> {
        todo!()
    }
}
