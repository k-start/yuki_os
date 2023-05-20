#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]

use bootloader_api::BootInfo;
use x86_64::{structures::paging::PageTableFlags, VirtAddr};

#[macro_use]
pub mod print;

pub mod ata;
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

    // let device = fs::ata_wrapper::AtaWrapper::new(0);
    // let cont = fat32::volume::Volume::new(device);
    // let mut root = cont.root_dir();
    // root.create_file("test2.txt").unwrap();

    let memory_info = unsafe { memory::MEMORY_INFO.as_mut().unwrap() };

    let userspace_fn_1_in_kernel = VirtAddr::new(userspace_prog_1 as *const () as u64);
    let userspace_fn_phys = unsafe {
        memory::translate_addr(userspace_fn_1_in_kernel, memory_info.phys_mem_offset).unwrap()
    };
    let page_phys_start = (userspace_fn_phys.as_u64() >> 12) << 12;
    let fn_page_offset = userspace_fn_phys.as_u64() - page_phys_start;
    let userspace_fn_virt_base = 0x400000;
    let userspace_fn_virt = VirtAddr::new(userspace_fn_virt_base + fn_page_offset);
    println!("{:?}", userspace_fn_virt);

    println!(
        "Mapping {:x} to {:x}",
        page_phys_start, userspace_fn_virt_base
    );

    let (user_page_table_ptr, user_page_table_physaddr) = memory::create_new_user_pagetable();

    memory::allocate_pages(
        user_page_table_ptr,
        VirtAddr::new(userspace_fn_virt_base),
        0x20000 as u64, // Size (bytes)
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
    )
    .expect("Could not allocate memory");

    memory::allocate_pages(
        user_page_table_ptr,
        VirtAddr::new(0x800000),
        0x20000 as u64, // Size (bytes)
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
    )
    .expect("Could not allocate memory");

    memory::switch_to_pagetable(user_page_table_physaddr);

    let input_ptr: *const u8 =
        VirtAddr::new((userspace_fn_1_in_kernel.as_u64() >> 12) << 12).as_ptr();
    let dest_ptr: *const u8 = VirtAddr::new(0x400000).as_ptr();
    for i in 0..0x20000 {
        unsafe {
            let in_ptr = input_ptr.add(i);
            let value = core::ptr::read_unaligned(in_ptr);

            let out_ptr: *mut u8 = dest_ptr.add(i).cast_mut();
            core::ptr::write(out_ptr, value);

            // println!("{:?} {:?} {:?}", in_ptr, out_ptr, value);
        }
    }

    jmp_to_usermode(userspace_fn_virt, VirtAddr::new(0x801000));
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

#[allow(named_asm_labels)]
#[inline(always)]
pub unsafe fn userspace_prog_1() {
    core::arch::asm!(
        "\
    start:
    mov rax, 0xCA11
    mov rdi, 10
    mov rsi, 20
    mov rdx, 30
    mov r10, 40
    syscall
    jmp start
"
    );
}

// #[allow(named_asm_labels)]
// #[inline(always)]
// pub unsafe fn userspace_prog_1() {
//     core::arch::asm!(
//         "\
//     nop
//     nop
//     nop
// "
//     );
// }

#[inline(never)]
pub fn jmp_to_usermode(code: VirtAddr, stack_end: VirtAddr) {
    unsafe {
        let (cs_idx, ds_idx) = gdt::set_usermode_segments();
        x86_64::instructions::tlb::flush_all(); // flush the TLB after address-space switch

        core::arch::asm!("\
        push rax   // stack segment
        push rsi   // rsp
        push 0x200 // rflags (only interrupt bit set)
        push rdx   // code segment
        push rdi   // ret to virtual addr
        iretq",
        in("rdi") code.as_u64(), in("rsi") stack_end.as_u64(), in("dx") cs_idx, in("ax") ds_idx);
    }
}
