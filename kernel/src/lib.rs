#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(asm_const)]

use bootloader_api::BootInfo;
use fs::devfs::DevFs;

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
pub mod mouse;
pub mod process;
pub mod scheduler;
pub mod syscalls;

extern crate alloc;

pub fn init(boot_info: &'static mut BootInfo) {
    // iniitialize drivers
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

    // Load ram disk and mount relevant virtual filesystems into memory
    let ramdisk_addr = boot_info.ramdisk_addr.into_option().unwrap() as *const u8;
    let initrd = unsafe { fs::initrd::InitRd::new(ramdisk_addr, boot_info.ramdisk_len as usize) };
    fs::vfs::mount("initrd", initrd);

    let stdiofs = StdioFs::new();
    fs::vfs::mount("stdio", stdiofs);

    let devfs = DevFs::new();
    fs::vfs::mount("dev", devfs);
    mouse::init_mouse();

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let framebufferfs = fs::framebuffer::FrameBufferFs::new(framebuffer);
        fs::vfs::mount("framebuffer", framebufferfs);
    }

    println!("{:?}", fs::vfs::list_dir("/framebuffer"));

    // let device = fs::fat32ata::Fat32Ata::new(0);
    // let fs = fs::fatfs::FatFs::new(device);

    // fs::vfs::mount(fs);
    // let file = fs::vfs::open("a:/test-binary").unwrap();

    // load  memory manager application and schedule it
    let file2 = fs::vfs::open("/initrd/window-manager").unwrap();

    let sched = &scheduler::SCHEDULER.read();
    // sched.schedule(file);
    sched.schedule(file2);

    println!("{:?}", fs::vfs::list_dir("/stdio/1"));

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
