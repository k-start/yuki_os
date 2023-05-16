#![no_std]
#![no_main]

#[macro_use]
pub mod print;

use bootloader_api::{
    config::{BootloaderConfig, Mapping},
    BootInfo,
};
use core::panic::PanicInfo;
use x86_64::VirtAddr;

extern crate alloc;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

bootloader_api::entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    yuki_os_lib::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { yuki_os_lib::memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { yuki_os_lib::memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    yuki_os_lib::memory::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

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
    yuki_os_lib::hlt_loop();
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    yuki_os_lib::hlt_loop();
}
