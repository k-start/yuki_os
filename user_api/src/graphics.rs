use crate::framebuffer::{Display, FrameBuffer, PixelFormat};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{Rgb888, RgbColor},
    Pixel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl FrameBuffer {
    pub fn clear_to_color(&mut self, color: Color) {
        let info = self.info();
        let bpp = info.bytes_per_pixel;
        let stride_bytes = info.stride * bpp;
        let width_bytes = info.width * bpp;
        let buf = self.buffer_mut();

        let (byte0, byte1, byte2) = match info.pixel_format {
            PixelFormat::Rgb => (color.red, color.green, color.blue),
            PixelFormat::Bgr => (color.blue, color.green, color.red),
            _ => (0, 0, 0),
        };

        for y in 0..info.height {
            let line_start = y * stride_bytes;
            let line_end = line_start + width_bytes;
            if line_end <= buf.len() {
                let line_slice = &mut buf[line_start..line_end];
                if bpp == 3 {
                    let mut i = 0;
                    while i + 2 < line_slice.len() {
                        line_slice[i] = byte0;
                        line_slice[i + 1] = byte1;
                        line_slice[i + 2] = byte2;
                        i += 3;
                    }
                } else if bpp == 4 {
                    let mut i = 0;
                    while i + 3 < line_slice.len() {
                        line_slice[i] = byte0;
                        line_slice[i + 1] = byte1;
                        line_slice[i + 2] = byte2;
                        line_slice[i + 3] = 0;
                        i += 4;
                    }
                } else {
                    for chunk in line_slice.chunks_exact_mut(bpp) {
                        chunk[0] = byte0;
                        if bpp > 1 {
                            chunk[1] = byte1;
                        }
                        if bpp > 2 {
                            chunk[2] = byte2;
                        }
                    }
                }
            }
        }
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        let info = self.info();
        let bpp = info.bytes_per_pixel;
        let stride_bytes = info.stride * bpp;

        let (byte0, byte1, byte2) = match info.pixel_format {
            PixelFormat::Rgb => (color.red, color.green, color.blue),
            PixelFormat::Bgr => (color.blue, color.green, color.red),
            _ => (0, 0, 0),
        };

        let x_start = x * bpp;
        let rect_width_bytes = w * bpp;
        let buf = self.buffer_mut();

        let y_end = core::cmp::min(y + h, info.height);
        let x_end_bytes = x_start + rect_width_bytes;

        for cur_y in y..y_end {
            let line_start = cur_y * stride_bytes;
            let start = line_start + x_start;
            let end = line_start + x_end_bytes;
            if end <= buf.len() {
                let dest = &mut buf[start..end];
                if bpp == 3 {
                    let mut i = 0;
                    while i + 2 < dest.len() {
                        dest[i] = byte0;
                        dest[i + 1] = byte1;
                        dest[i + 2] = byte2;
                        i += 3;
                    }
                } else if bpp == 4 {
                    let mut i = 0;
                    while i + 3 < dest.len() {
                        dest[i] = byte0;
                        dest[i + 1] = byte1;
                        dest[i + 2] = byte2;
                        dest[i + 3] = 0;
                        i += 4;
                    }
                } else {
                    for chunk in dest.chunks_exact_mut(bpp) {
                        chunk[0] = byte0;
                        if bpp > 1 {
                            chunk[1] = byte1;
                        }
                        if bpp > 2 {
                            chunk[2] = byte2;
                        }
                    }
                }
            }
        }
    }

    pub fn draw_rect_clipped(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        let info = self.info();
        let screen_w = info.width as i32;
        let screen_h = info.height as i32;

        let w_i32 = w as i32;
        let h_i32 = h as i32;

        // Check if the rectangle is entirely off-screen
        if x >= screen_w || y >= screen_h || x + w_i32 <= 0 || y + h_i32 <= 0 {
            return;
        }

        // Clip to screen boundaries
        let clip_x = core::cmp::max(0, x);
        let clip_y = core::cmp::max(0, y);
        let clip_w = core::cmp::min(x + w_i32, screen_w) - clip_x;
        let clip_h = core::cmp::min(y + h_i32, screen_h) - clip_y;

        if clip_w > 0 && clip_h > 0 {
            self.draw_rect(
                clip_x as usize,
                clip_y as usize,
                clip_w as usize,
                clip_h as usize,
                color,
            );
        }
    }
}

#[inline(always)]
fn draw_pixels_format<I, F>(
    pixels: I,
    width: usize,
    height: usize,
    stride_bytes: usize,
    bpp: usize,
    ptr: *mut u8,
    write_pixel: F,
) where
    I: IntoIterator<Item = Pixel<Rgb888>>,
    F: Fn(*mut u8, Rgb888),
{
    for Pixel(coordinates, color) in pixels {
        let c: (i32, i32) = coordinates.into();
        let x = c.0 as usize;
        let y = c.1 as usize;
        if x < width && y < height {
            let byte_offset = y * stride_bytes + x * bpp;
            unsafe {
                write_pixel(ptr.add(byte_offset), color);
            }
        }
    }
}

impl<'f> DrawTarget for Display<'f> {
    type Color = Rgb888;

    /// Drawing operations can never fail.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let info = self.framebuffer.info();
        let width = info.width;
        let height = info.height;
        let bpp = info.bytes_per_pixel;
        let stride_bytes = info.stride * bpp;
        let format = info.pixel_format;

        let ptr = self.framebuffer.buffer_mut().as_mut_ptr();

        match format {
            PixelFormat::Rgb => {
                draw_pixels_format(
                    pixels,
                    width,
                    height,
                    stride_bytes,
                    bpp,
                    ptr,
                    |p, color| unsafe {
                        *p = color.r();
                        *p.add(1) = color.g();
                        *p.add(2) = color.b();
                    },
                );
            }
            PixelFormat::Bgr => {
                draw_pixels_format(
                    pixels,
                    width,
                    height,
                    stride_bytes,
                    bpp,
                    ptr,
                    |p, color| unsafe {
                        *p = color.b();
                        *p.add(1) = color.g();
                        *p.add(2) = color.r();
                    },
                );
            }
            PixelFormat::U8 => {
                draw_pixels_format(
                    pixels,
                    width,
                    height,
                    stride_bytes,
                    bpp,
                    ptr,
                    |p, color| unsafe {
                        *p = color.r() / 3 + color.g() / 3 + color.b() / 3;
                    },
                );
            }
            _ => {
                draw_pixels_format(
                    pixels,
                    width,
                    height,
                    stride_bytes,
                    bpp,
                    ptr,
                    |p, color| unsafe {
                        *p = color.r();
                        if bpp > 1 {
                            *p.add(1) = color.g();
                        }
                        if bpp > 2 {
                            *p.add(2) = color.b();
                        }
                    },
                );
            }
        }

        Ok(())
    }

    fn fill_solid(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        let color = Color {
            red: color.r(),
            green: color.g(),
            blue: color.b(),
        };
        self.framebuffer.draw_rect_clipped(
            area.top_left.x,
            area.top_left.y,
            area.size.width,
            area.size.height,
            color,
        );
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let color = Color {
            red: color.r(),
            green: color.g(),
            blue: color.b(),
        };
        self.framebuffer.clear_to_color(color);
        Ok(())
    }
}

impl<'f> OriginDimensions for Display<'f> {
    fn size(&self) -> Size {
        let info = self.framebuffer.info();

        Size::new(info.width as u32, info.height as u32)
    }
}
