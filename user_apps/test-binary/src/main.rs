#![no_std]
#![no_main]

#[macro_use]
extern crate user_api;

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

#[no_mangle]
fn main() {
    // unsafe {
    // let fb_info: FrameBufferInfo = FrameBufferInfo::default();

    // let fd = user_api::syscalls::open(b"/framebuffer/0");
    // let ptr: *const FrameBufferInfo = &fb_info as *const FrameBufferInfo;
    // user_api::syscalls::ioctl(fd, 0, ptr as usize);

    // println!("{:?}", fb_info);

    // let framebuffer = user_api::syscalls::mmap(0, fb_info.byte_len, fd) as *mut u8;

    // unsafe {
    //     for r in 0..255 {
    //         for i in (0..fb_info.byte_len).step_by(3) {
    //             (*framebuffer.wrapping_add(i)) = 0;
    //             (*framebuffer.wrapping_add(i + 1)) = 0;
    //             (*framebuffer.wrapping_add(i + 2)) = r;
    //         }
    //     }
    // }

    // let mut buf = [255; 512];
    // user_api::syscalls::write(fd, &mut buf);
    // println!("{buf:?}");
    // }
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
