// Filesystem for storing the framebuffer for applications to draw to the screen
use super::filesystem::{Error, File};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use bootloader_api::info::FrameBuffer;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum PixelFormat {
    /// One byte red, then one byte green, then one byte blue.
    ///
    /// Length might be larger than 3, check [`bytes_per_pixel`][FrameBufferInfo::bytes_per_pixel]
    /// for this.
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

pub struct FrameBufferFs<'a> {
    framebuffer: &'a FrameBuffer,
}

impl FrameBufferFs<'static> {
    pub const fn new(framebuffer: &'static FrameBuffer) -> FrameBufferFs<'static> {
        FrameBufferFs { framebuffer }
    }

    fn generate_info(&self) -> String {
        let pixel_format = match self.framebuffer.info().pixel_format {
            bootloader_api::info::PixelFormat::Rgb => PixelFormat::Rgb,
            bootloader_api::info::PixelFormat::Bgr => PixelFormat::Bgr,
            bootloader_api::info::PixelFormat::U8 => PixelFormat::U8,
            bootloader_api::info::PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            } => PixelFormat::Unknown {
                red_position,
                green_position,
                blue_position,
            },
            _ => todo!(),
        };
        let info = FrameBufferInfo {
            byte_len: self.framebuffer.info().byte_len,
            width: self.framebuffer.info().width,
            height: self.framebuffer.info().height,
            pixel_format,
            bytes_per_pixel: self.framebuffer.info().bytes_per_pixel,
            stride: self.framebuffer.info().stride,
        };
        serde_json::to_string(&info).unwrap()
    }
}

impl super::filesystem::FileSystem for FrameBufferFs<'static> {
    fn dir_entries(&self, _dir: &str) -> Result<Vec<File>, Error> {
        let mut vec: Vec<File> = Vec::new();
        vec.push(File {
            name: "0".to_string(),
            path: "0".to_string(),
            r#type: "file".to_string(),
            size: self.framebuffer.info().byte_len as u64,
            ptr: None,
        });
        vec.push(File {
            name: "0-info".to_string(),
            path: "0-info".to_string(),
            r#type: "file".to_string(),
            size: self.generate_info().len() as u64,
            ptr: None,
        });
        Ok(vec)
    }

    fn open(&self, path: &str) -> Result<File, Error> {
        if path.contains("info") {
            return Ok(File {
                name: "0-info".to_string(),
                path: path.to_string(),
                r#type: "file".to_string(),
                size: self.generate_info().len() as u64,
                ptr: None,
            });
        }
        Ok(File {
            name: "0".to_string(),
            path: path.to_string(),
            r#type: "file".to_string(),
            size: self.framebuffer.info().byte_len as u64,
            ptr: None,
        })
    }

    fn read(&self, file: &File, buf: &mut [u8]) -> Result<(), Error> {
        if file.name.contains("info") {
            let info_str = self.generate_info();
            let info = info_str.as_bytes();
            buf.copy_from_slice(&info);
        } else {
            let fb = self.framebuffer.buffer();
            buf.copy_from_slice(&fb[..buf.len()]);
        }
        Ok(())
    }

    fn write(&self, file: &File, buf: &[u8]) -> Result<(), Error> {
        if !file.name.contains("info") {
            let pointer = self.framebuffer.buffer().as_ptr();
            let fb = unsafe {
                core::slice::from_raw_parts_mut(
                    pointer as *mut u8,
                    self.framebuffer.info().byte_len,
                )
            };
            fb[..buf.len()].copy_from_slice(buf);
        }
        Ok(())
    }
}
