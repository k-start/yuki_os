#![no_std]
#![no_main]

#[macro_use]
extern crate user_api;

#[no_mangle]
fn main() {
    unsafe {
        user_api::syscalls::open(b"stdin");
    };
    loop {

        // println!("hello from app");
    }
}
