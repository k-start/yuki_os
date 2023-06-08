#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]

use bootloader_api::BootInfo;
use elfloader::ElfBinary;
use x86_64::{structures::paging::PageTableFlags, VirtAddr};

#[macro_use]
pub mod print;

pub mod ata;
pub mod elf;
pub mod fs;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod syscalls;

extern crate alloc;

pub fn init(boot_info: &'static mut BootInfo) {
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

    let (user_page_table_ptr, user_page_table_physaddr) = memory::create_new_user_pagetable();

    memory::switch_to_pagetable(user_page_table_physaddr);

    unsafe {
        memory::allocate_pages(
            user_page_table_ptr,
            VirtAddr::new(0x500000000000),
            file.size as u64,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        )
        .expect("Could not allocate memory");
    }

    // fix me - terrible loading
    let file_buf: &mut [u8] =
        unsafe { core::slice::from_raw_parts_mut(0x500000000000 as *mut u8, file.size as usize) };
    fs::vfs::read(&file, file_buf);

    let binary = ElfBinary::new(file_buf).unwrap();
    let mut loader = elf::loader::UserspaceElfLoader {
        vbase: 0x400000,
        user_page_table_ptr,
    };
    binary.load(&mut loader).expect("Can't load the binary");

    // user heap
    unsafe {
        memory::allocate_pages(
            user_page_table_ptr,
            VirtAddr::new(0x800000),
            0x1000_u64,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        )
        .expect("Could not allocate memory");
    }

    jmp_to_usermode(
        VirtAddr::new(loader.vbase + binary.entry_point()),
        VirtAddr::new(0x801000),
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

#[inline(never)]
pub fn jmp_to_usermode(code: VirtAddr, stack_end: VirtAddr) {
    unsafe {
        let (cs_idx, ds_idx) = gdt::set_usermode_segments();
        x86_64::instructions::tlb::flush_all(); // flush the TLB after address-space switch

        core::arch::asm!(
            "cli",        // Disable interrupts
            "push {:r}",  // Stack segment (SS)
            "push {:r}",  // Stack pointer (RSP)
            "push 0x200", // RFLAGS with interrupts enabled
            "push {:r}",  // Code segment (CS)
            "push {:r}",  // Instruction pointer (RIP)
            "iretq",
            in(reg) ds_idx,
            in(reg) stack_end.as_u64(),
            in(reg) cs_idx,
            in(reg) code.as_u64(),
        );
    }
}
