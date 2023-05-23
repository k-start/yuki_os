#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]

use bootloader_api::BootInfo;
use object::{Object, ObjectSegment};
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

    // println!("{},{},{},{},{}", b'F', b'A', b'T', b'3', b'2');

    // for i in 0..10000 {
    //     let mut buf: [u8; 512] = [0; 512];
    //     ata::read(0, i, &mut buf);
    //     for j in buf {
    //         if j == b'F' {
    //             println!("{i} - {:?}", buf);
    //             break;
    //         }
    //     }
    // }

    let device = fs::ata_wrapper::AtaWrapper::new(0);
    let cont = fat32::volume::Volume::new(device);
    let root = cont.root_dir();
    let file = root.open_file("test-binary").unwrap();

    let mut file_buf: [u8; 0x2000] = [0; 0x2000];
    let _size = file.read(&mut file_buf).unwrap();

    const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

    if file_buf[0..4] != ELF_MAGIC {
        panic!("Expected ELF binary");
    }

    if let Ok(obj) = object::File::parse(&file_buf[..]) {
        // Mount app at 0x400000 in memory
        let userspace_fn_virt_base = 0x400000;

        let (user_page_table_ptr, user_page_table_physaddr) = memory::create_new_user_pagetable();

        // user heap
        memory::allocate_pages(
            user_page_table_ptr,
            VirtAddr::new(0x800000),
            0x1000 as u64, // Size (bytes)
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        )
        .expect("Could not allocate memory");

        // entry point + 0x400000
        let entry_point = userspace_fn_virt_base + obj.entry();
        for segment in obj.segments() {
            // segment address + 0x400000
            let segment_address = userspace_fn_virt_base + segment.address();
            let start_address = VirtAddr::new(segment_address);

            // allocate pages for the app
            memory::allocate_pages(
                user_page_table_ptr,
                start_address,
                segment.size() as u64,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            )
            .expect("Could not allocate memory");

            memory::switch_to_pagetable(user_page_table_physaddr);

            if let Ok(data) = segment.data() {
                let dest_ptr = segment_address as *mut u8;
                for (i, value) in data.iter().enumerate() {
                    unsafe {
                        let ptr = dest_ptr.add(i);
                        core::ptr::write(ptr, *value);
                    }
                }
            }
        }

        jmp_to_usermode(VirtAddr::new(entry_point), VirtAddr::new(0x801000));
    }
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
