#![no_std]
#![no_main]

use core::alloc::GlobalAlloc;

use serde::Deserialize;

#[macro_use]
extern crate user_api;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
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

#[no_mangle]
fn main() {
    unsafe {
        // let fd = user_api::syscalls::open(b"/framebuffer/0-info");
        // let mut buf: [u8; 101] = [0; 101];
        // user_api::syscalls::read(fd, &mut buf);
        // let string = core::str::from_utf8(&buf).unwrap();
        // let (info, _size): (FrameBufferInfo, usize) = serde_json_core::from_str(string).unwrap();
        // println!("{info:?}");

        // let fd = user_api::syscalls::open(b"/framebuffer/0");
        // buf = [255; 101];
        // user_api::syscalls::write(fd, &mut buf);
        // println!("{buf:?}");

        let fb_info: FrameBufferInfo = FrameBufferInfo::default();

        let fd = user_api::syscalls::open(b"/framebuffer/0");
        let ptr: *const FrameBufferInfo = &fb_info as *const FrameBufferInfo;
        user_api::syscalls::ioctl(fd, 0, ptr as usize);

        println!("{:?}", fb_info);
    }
    loop {
        let mut x: [u8; 1] = [0; 1];
        unsafe {
            user_api::syscalls::read(0, &mut x);
        };
        if x != [0] {
            print!("{}", x[0] as char);
        }
    }
}

// For some reason we have to compile with an allocator for serde, even with no_std
// It never actually allocs
#[global_allocator]
static ALLOCATOR: TestAllocator = TestAllocator {};

struct TestAllocator {}

unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        todo!()
    }
}
