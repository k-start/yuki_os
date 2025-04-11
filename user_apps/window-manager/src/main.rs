#![no_std]
#![no_main]

use event::mouseevent::MOUSE_EVENT;
use world::WORLD;

#[macro_use]
extern crate user_api;

extern crate alloc;

mod event;
mod framebuffer;
mod mouse;
mod window;
mod windowmanager;
mod world;

#[no_mangle]
fn main() {
    windowmanager::WindowManager::new();
    mouse::Mouse::new();

    // let mut x = 0;
    // let mut y = 0;
    // let mut stdin_buf: [u8; 1] = [0; 1];
    // let mut str_buf: [u8; 2] = [0; 2];

    loop {
        MOUSE_EVENT.lock().poll();

        WORLD.lock().render();

        // let bytes_read = unsafe { user_api::syscalls::read(0, &mut stdin_buf) };

        // if bytes_read > 0 {
        //     if stdin_buf[0] == '\n' as u8 {
        //         y = y + 1;
        //         x = 0;
        //         continue;
        //     }
        //     if stdin_buf[0] == '\x08' as u8 {
        //         x = x - 1;
        //         Rectangle::new(
        //             Point::new(10, 15) + Point::new(12 * x, 22 * y),
        //             Size::new(10, 20),
        //         )
        //         .into_styled(background_style)
        //         .draw(&mut display)
        //         .unwrap();
        //         continue;
        //     }
        //     Text::new(
        //         (stdin_buf[0] as char).encode_utf8(&mut str_buf),
        //         Point::new(10, 30) + Point::new(12 * x, 22 * y),
        //         character_style,
        //     )
        //     .draw(&mut display)
        //     .unwrap();
        //     x = x + 1;
        // }
    }
}
