#![no_std]
#![no_main]

use embedded_graphics::{
    mock_display::MockDisplay,
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::{BinaryColor, Rgb888},
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
};
use framebuffer::Display;

#[macro_use]
extern crate user_api;

mod framebuffer;

#[no_mangle]
fn main() {
    let fd = unsafe { user_api::syscalls::open(b"/framebuffer/0") };

    let mut fb = framebuffer::FrameBuffer::new(fd);
    fb.clear();
    let mut display = Display::new(&mut fb);

    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb888::WHITE)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    let character_style = MonoTextStyle::new(&FONT_10X20, Rgb888::WHITE);

    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(&mut display)
        .unwrap();

    let mut i = 0;
    let mut y = 0;
    let mut x: [u8; 1] = [0; 1];
    let mut str: [u8; 2] = [0; 2];

    loop {
        unsafe {
            user_api::syscalls::read(0, &mut x);
        };

        if x != [0] {
            if x == ['\n' as u8] {
                y = y + 22;
                i = 0;
                continue;
            }
            Text::with_alignment(
                (x[0] as char).encode_utf8(&mut str),
                display.bounding_box().center() + Point::new(12 * i, y),
                character_style,
                Alignment::Center,
            )
            .draw(&mut display)
            .unwrap();
            i = i + 1;
        }
    }
}
