#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]

use bootloader_api::BootInfo;

#[macro_use]
pub mod print;

pub mod ata;
pub mod elf;
pub mod fs;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod scheduler;
pub mod syscalls;

extern crate alloc;

pub fn init(boot_info: &'static mut BootInfo) {
    x86_64::instructions::interrupts::disable();
    gdt::init();
    interrupts::init();
    memory::init(
        boot_info.physical_memory_offset.into_option(),
        &boot_info.memory_regions,
    );
    ata::init();
    syscalls::init();
    fs::vfs::init();

    let device = fs::fat32ata::Fat32Ata::new(0);
    let fs = fs::fatfs::FatFs::new(device);

    fs::vfs::mount(fs);
    let file = fs::vfs::open("a:/test-binary").unwrap();
    let file2 = fs::vfs::open("a:/hello-world").unwrap();

    let sched = &scheduler::SCHEDULER;
    sched.schedule(file);
    sched.schedule(file2);
    x86_64::instructions::interrupts::enable();
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
