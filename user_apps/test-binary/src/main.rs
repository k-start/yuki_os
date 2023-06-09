#![no_std]
#![no_main]

#[macro_use]
extern crate user_api;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() {
    let mut i = 0;
    loop {
        println!("{}", i);
        i += 1;
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
