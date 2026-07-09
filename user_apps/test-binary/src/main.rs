#![no_std]
#![no_main]

use user_api::window::Window;

#[macro_use]
extern crate user_api;

#[no_mangle]
fn main() {
    let window = Window::new(100, 100, 400, 400).unwrap();

    let width = window.width as usize;
    let height = window.height as usize;
    let bpp = window.bytes_per_pixel as usize;

    for y in 0..height {
        for x in 0..width {
            let r = (x % 255) as u8;
            let g = (y % 255) as u8;
            let b = ((x + y) % 255) as u8;
            let offset = (y * width + x) * bpp;
            window.buffer[offset] = r;
            window.buffer[offset + 1] = g;
            window.buffer[offset + 2] = b;
        }
    }
}
