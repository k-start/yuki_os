#![no_std]
#![no_main]

use event::mouseevent::MOUSE_EVENT;
use world::WORLD;

#[macro_use]
extern crate user_api;

extern crate alloc;

mod event;
mod framebuffer;
mod graphics;
mod window;
mod windowmanager;
mod world;

#[no_mangle]
fn main() {
    loop {
        while let Some(e) = MOUSE_EVENT.lock().poll() {
            WORLD.lock().handle_mouse_event(e);
        }

        WORLD.lock().render();
    }
}
