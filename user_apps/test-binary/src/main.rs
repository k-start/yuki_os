#![no_std]
#![no_main]

use user_api::window::Window;

#[macro_use]
extern crate user_api;

#[no_mangle]
fn main() {
    let _window = Window::new(100, 100, 400, 400).unwrap();
}
