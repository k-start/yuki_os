#![no_std]
#![no_main]

#[macro_use]
extern crate user_api;
use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() {
    // let string = "hello from app";
    loop {
        println!("println");
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
