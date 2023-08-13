#![no_std]

#[macro_use]
pub mod print;
pub mod syscalls;

use core::panic::PanicInfo;
#[panic_handler]
pub fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        syscalls::exit();
    };
    loop {}
}

extern "C" {
    fn main() -> ();
}

#[no_mangle]
pub unsafe extern "C" fn _start() {
    #[cfg(not(test))]
    main();
    syscalls::exit();
}
