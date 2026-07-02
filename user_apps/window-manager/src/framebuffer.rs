#![allow(dead_code)]
use core::slice;

#[derive(Debug, Clone, Copy, Default)]
pub struct FrameBufferInfo {
    /// The total size in bytes.
    pub byte_len: usize,
    /// The width in pixels.
    pub width: usize,
    /// The height in pixels.
    pub height: usize,
    /// The color format of each pixel.
    pub pixel_format: PixelFormat,
    /// The number of bytes per pixel.
    pub bytes_per_pixel: usize,
    /// Number of pixels between the start of a line and the start of the next.
    ///
    /// Some framebuffers use additional padding at the end of a line, so this
    /// value might be larger than `horizontal_resolution`. It is
    /// therefore recommended to use this field for calculating the start address of a line.
    pub stride: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// One byte red, then one byte green, then one byte blue.
    ///
    /// Length might be larger than 3, check [`bytes_per_pixel`][FrameBufferInfo::bytes_per_pixel]
    /// for this.
    #[default]
    Rgb,
    /// One byte blue, then one byte green, then one byte red.
    ///
    /// Length might be larger than 3, check [`bytes_per_pixel`][FrameBufferInfo::bytes_per_pixel]
    /// for this.
    Bgr,
    /// A single byte, representing the grayscale value.
    ///
    /// Length might be larger than 1, check [`bytes_per_pixel`][FrameBufferInfo::bytes_per_pixel]
    /// for this.
    U8,
    /// Unknown pixel format.
    Unknown {
        /// Bit offset of the red value.
        red_position: u8,
        /// Bit offset of the green value.
        green_position: u8,
        /// Bit offset of the blue value.
        blue_position: u8,
    },
}

pub struct FrameBuffer {
    info: FrameBufferInfo,
    addr: usize,
    back_buffer: alloc::vec::Vec<u8>,
}

impl FrameBuffer {
    pub fn new(fd: usize) -> Self {
        let info: FrameBufferInfo = FrameBufferInfo::default();

        let ptr: *const FrameBufferInfo = &info as *const FrameBufferInfo;

        unsafe {
            user_api::syscalls::ioctl(fd, 0, ptr as usize);
        }

        let framebuffer = unsafe { user_api::syscalls::mmap(0, info.byte_len, fd) };

        let back_buffer = alloc::vec![0; info.byte_len];

        Self {
            info,
            addr: framebuffer,
            back_buffer,
        }
    }

    pub fn clear(&mut self) {
        for i in self.buffer_mut() {
            *i = 0;
        }
    }

    pub fn flush(&mut self) {
        unsafe {
            let front = slice::from_raw_parts_mut(self.addr as *mut u8, self.info.byte_len);
            front.copy_from_slice(&self.back_buffer);
        }
    }

    pub fn info(&self) -> FrameBufferInfo {
        self.info
    }

    pub(crate) fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.back_buffer
    }

    pub(crate) fn buffer(&self) -> &[u8] {
        &self.back_buffer
    }
}

pub struct Display<'f> {
    pub(crate) framebuffer: &'f mut FrameBuffer,
}

impl<'f> Display<'f> {
    pub fn new(framebuffer: &'f mut FrameBuffer) -> Display {
        Display { framebuffer }
    }
}
