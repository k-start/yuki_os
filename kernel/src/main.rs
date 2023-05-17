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
    kernel_lib::init(boot_info);
    println!("Welcome to Yuki OS");
    kernel_lib::hlt_loop();
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernel_lib::hlt_loop();
}
