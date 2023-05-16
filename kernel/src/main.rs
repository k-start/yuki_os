#![no_std]
#![no_main]

#[macro_use]
pub mod print;

use bootloader_api::{
    config::{BootloaderConfig, Mapping},
    BootInfo,
};
use core::panic::PanicInfo;

extern crate alloc;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

bootloader_api::entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel_lib::init();
    kernel_lib::memory::init(
        boot_info.physical_memory_offset.into_option(),
        &boot_info.memory_regions,
    );

    let x = alloc::boxed::Box::new(41);
    println!("heap_value at {:p}", x);

    println!("Welcome to Yuki OS");

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let mut value = 0x90;
        for byte in framebuffer.buffer_mut() {
            *byte = value;
            value = value.wrapping_add(1);
        }
    }
    kernel_lib::hlt_loop();
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernel_lib::hlt_loop();
}
