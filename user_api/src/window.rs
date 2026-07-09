use crate::syscalls::{get_pid, mmap, open, read, write};
use alloc::format;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum PixelFormat {
    Rgb,
    Bgr,
    U8,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub pid: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CreateResponse {
    pub window_id: u32,
    pub buffer_size: u32,
    pub bytes_per_pixel: u8,
    pub pixel_format: PixelFormat,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum WindowCommand {
    Create(CreateRequest),
}
pub struct Window {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub bytes_per_pixel: u8,
    pub pixel_format: PixelFormat,
    pub buffer: &'static mut [u8],
}

impl Window {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Result<Self, &'static str> {
        let pid = unsafe { get_pid() } as u32;
        let cmd = WindowCommand::Create(CreateRequest {
            x,
            y,
            width,
            height,
            pid,
        });
        let controller_fd = unsafe { open(b"/dev/wm_controller\0") };

        // Write window creation request to controller file
        let cmd_slice = unsafe {
            core::slice::from_raw_parts(
                &cmd as *const WindowCommand as *const u8,
                core::mem::size_of::<WindowCommand>(),
            )
        };
        unsafe { write(controller_fd, cmd_slice) };

        // Read response from window manager
        let response_fd = unsafe { open(format!("/dev/wm_response_{pid}\0").as_bytes()) };
        let mut response = CreateResponse {
            window_id: 0,
            buffer_size: 0,
            bytes_per_pixel: 0,
            pixel_format: PixelFormat::Unknown,
        };

        let response_size = core::mem::size_of::<CreateResponse>();
        let response_slice = unsafe {
            core::slice::from_raw_parts_mut(
                &mut response as *mut CreateResponse as *mut u8,
                response_size,
            )
        };

        // We have to loop here until we receive the response
        // One day I will implement better io
        let mut total_read = 0;
        while total_read < response_size {
            let bytes_read = unsafe { read(response_fd, &mut response_slice[total_read..]) };
            if bytes_read > 0 {
                total_read += bytes_read as usize;
            }
        }

        let buf_fd =
            unsafe { open(format!("/dev/window_buf_{}\0", response.window_id).as_bytes()) };
        let mmap_addr = unsafe { mmap(0, response.buffer_size as usize, buf_fd) };

        let buffer = unsafe {
            core::slice::from_raw_parts_mut(mmap_addr as *mut u8, response.buffer_size as usize)
        };

        Ok(Self {
            id: response.window_id,
            width,
            height,
            bytes_per_pixel: response.bytes_per_pixel,
            pixel_format: response.pixel_format,
            buffer,
        })
    }
}
