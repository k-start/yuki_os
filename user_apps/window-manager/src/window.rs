use crate::framebuffer::Display;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
};

pub struct Window {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub bytes_per_pixel: u8,
    pub buffer: &'static mut [u8],
}

impl Window {
    pub fn new(
        id: u32,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        bytes_per_pixel: u8,
        buffer: &'static mut [u8],
    ) -> Self {
        Self {
            id,
            x,
            y,
            w,
            h,
            bytes_per_pixel,
            buffer,
        }
    }

    pub fn get_location(&self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }

    pub fn click(&self, x: i32, y: i32) {
        println!("window click {x}, {y}");
    }

    pub fn render(&mut self, display: &mut Display) {
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(Rgb888::WHITE)
            .stroke_width(3)
            .stroke_alignment(StrokeAlignment::Outside)
            .fill_color(Rgb888::BLACK)
            .build();

        Rectangle::new(Point::new(self.x, self.y), Size::new(self.w, self.h + 25))
            .into_styled(style)
            .draw(display)
            .unwrap();

        Rectangle::new(Point::new(self.x, self.y), Size::new(self.w, 25))
            .into_styled(style)
            .draw(display)
            .unwrap();

        Rectangle::new(
            Point::new(self.x + (self.w - 25) as i32, self.y),
            Size::new(25, 25),
        )
        .into_styled(style)
        .draw(display)
        .unwrap();

        let info = display.framebuffer.info();
        let fb_w = info.width as i32;
        let fb_h = info.height as i32;
        let bpp = info.bytes_per_pixel as usize;
        let stride_bytes = info.stride as usize * bpp;
        let fb_buf = display.framebuffer.buffer_mut();

        let client_y_start = self.y + 25;

        // Calculate clipping boundaries
        let y_start = core::cmp::max(0, -client_y_start) as usize + 3; // Add 3 to the y_start due to the stroke_width of the window border
        let y_end =
            (self.h as i32 - core::cmp::max(0, (client_y_start + self.h as i32) - fb_h)) as usize;

        let x_start = core::cmp::max(0, -self.x) as usize;
        let x_end = (self.w as i32 - core::cmp::max(0, (self.x + self.w as i32) - fb_w)) as usize;

        if y_start >= y_end || x_start >= x_end {
            return; // Completely clipped off-screen
        }

        // Copy line-by-line
        let win_bpp = self.bytes_per_pixel as usize;
        let copy_len = (x_end - x_start) * win_bpp;

        for y in y_start..y_end {
            let dest_y = (client_y_start + y as i32) as usize;
            let dest_line_offset = dest_y * stride_bytes + (self.x + x_start as i32) as usize * bpp;
            let src_line_offset = (y * self.w as usize + x_start) * win_bpp;

            if src_line_offset + copy_len <= self.buffer.len()
                && dest_line_offset + copy_len <= fb_buf.len()
            {
                fb_buf[dest_line_offset..dest_line_offset + copy_len]
                    .copy_from_slice(&self.buffer[src_line_offset..src_line_offset + copy_len]);
            }
        }
    }
}
