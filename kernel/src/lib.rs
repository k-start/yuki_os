#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]

use bootloader_api::BootInfo;

#[macro_use]
pub mod print;

pub mod gdt;
pub mod interrupts;
pub mod memory;

extern crate alloc;

pub fn init(boot_info: &'static mut BootInfo) {
    gdt::init();
    interrupts::init();
    memory::init(
        boot_info.physical_memory_offset.into_option(),
        &boot_info.memory_regions,
    );
}

pub fn outb(port: u16, val: u8) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(port);
    unsafe { port.write(val) };
}

pub fn inb(port: u16) -> u8 {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(port);
    let val: u8 = unsafe { port.read() };

    val
}

pub fn ins(port: u16) -> u16 {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(port);
    let val: u16 = unsafe { port.read() };

    val
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
