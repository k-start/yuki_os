#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(asm_const)]

use bootloader_api::BootInfo;

use crate::fs::stdio::StdioFs;

#[macro_use]
pub mod print;

pub mod ata;
pub mod elf;
pub mod fs;
pub mod gdt;
pub mod interrupts;
pub mod keyboard;
pub mod memory;
pub mod process;
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

    let ramdisk_addr = boot_info.ramdisk_addr.into_option().unwrap() as *const u8;
    let initrd = fs::initrd::InitRd::new(ramdisk_addr, boot_info.ramdisk_len as usize);
    fs::vfs::mount("initrd", initrd);

    let stdiofs = StdioFs::new();
    fs::vfs::mount("stdio", stdiofs);

    println!("{:?}", fs::vfs::list_dir("/initrd"));

    // let device = fs::fat32ata::Fat32Ata::new(0);
    // let fs = fs::fatfs::FatFs::new(device);

    // fs::vfs::mount(fs);
    // let file = fs::vfs::open("a:/test-binary").unwrap();
    let file2 = fs::vfs::open("/initrd/hello-world").unwrap();

    let sched = &scheduler::SCHEDULER;
    // sched.schedule(file);
    sched.schedule(file2);

    println!("{:?}", fs::vfs::list_dir("/stdio"));

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

// pub unsafe fn get_context() -> scheduler::Context {
//     let ctxp: *const scheduler::Context;
//     core::arch::asm!(
//         "push rbp",
//         "push rax",
//         "push rbx",
//         "push rcx",
//         "push rdx",
//         "push rsi",
//         "push rdi",
//         "push r8",
//         "push r9",
//         "push r10",
//         "push r11",
//         "push r12",
//         "push r13",
//         "push r14",
//         "push r15",
//         "mov {}, rsp",
//         "sub rsp, 0x400",
//     out(reg) ctxp);
//     let ret: scheduler::Context = core::ptr::read(ctxp);
//     ret
// }
