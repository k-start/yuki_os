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
        let mut x: [u8; 1] = [0; 1];
        unsafe {
            user_api::syscalls::read(0, &mut x);
        };
        if x != [0] {
            println!("{:?}", x);
        }
    }
}
