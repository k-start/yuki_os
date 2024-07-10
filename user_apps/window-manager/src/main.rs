#![no_std]
#![no_main]

use framebuffer::Color;

#[macro_use]
extern crate user_api;

mod framebuffer;

#[no_mangle]
fn main() {
    let fd = unsafe { user_api::syscalls::open(b"/framebuffer/0") };

    let mut fb = framebuffer::FrameBuffer::new(fd);

    let color = Color {
        red: 255,
        green: 0,
        blue: 0,
    };

    for x in 0..100 {
        for y in 0..100 {
            let position = framebuffer::Position {
                x: 20 + x,
                y: 100 + y,
            };
            framebuffer::set_pixel_in(&mut fb, position, color);
        }
    }

    loop {}
}
