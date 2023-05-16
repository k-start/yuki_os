#![no_std]
#![no_main]

use core::panic::PanicInfo;

use bootloader_api::BootInfo;

bootloader_api::entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    yuki_os_lib::init();

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let mut value = 0x90;
        for byte in framebuffer.buffer_mut() {
            *byte = value;
            value = value.wrapping_add(1);
        }
    }
    yuki_os_lib::hlt_loop();
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
