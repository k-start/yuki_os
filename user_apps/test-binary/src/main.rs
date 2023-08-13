#![no_std]
#![no_main]

#[macro_use]
extern crate user_api;

#[no_mangle]
fn main() {
    let mut i = 0;
    loop {
        println!("{}", i);
        i += 1;
    }
}
