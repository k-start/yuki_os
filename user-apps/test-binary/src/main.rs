#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() {
    let string = "hello from app";

    loop {
        unsafe {
            core::arch::asm!(
                "mov rax, 1",
                "mov rdi, 1",
                "syscall",
                in("rsi") string.as_ptr(),
                in("rdx") string.len()
            );
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
