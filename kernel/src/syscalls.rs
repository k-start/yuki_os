use core::{arch::asm, ffi::CStr};

use alloc::borrow::ToOwned;
use x86_64::VirtAddr;

use crate::{process::Context, scheduler, vfs};

const MSR_STAR: usize = 0xc0000081;
const MSR_LSTAR: usize = 0xc0000082;
const MSR_FMASK: usize = 0xc0000084;

const MSR_KERNEL_GS_BASE: usize = 0xC0000102;

pub fn init() {
    let handler_addr = wrapped_syscall_handler as *const () as u64;

    unsafe {
        asm!("mov ecx, 0xC0000080", "rdmsr", "or eax, 1", "wrmsr");

        asm!("xor rdx, rdx",
             "mov rax, 0x300",
             "wrmsr",
             in("rcx") MSR_FMASK);
        // write handler address to AMD's MSR_LSTAR register
        asm!("mov rdx, rax",
             "shr rdx, 32",
             "wrmsr",
             in("rax") handler_addr,
             in("rcx") MSR_LSTAR);
        // write segments to use on syscall/sysret to AMD'S MSR_STAR register
        asm!(
            "xor rax, rax",
            "mov rdx, 0x230008", // use seg selectors 8, 16 for syscall and 43, 51 for sysret
            "wrmsr",
            in("rcx") MSR_STAR);

        asm!(
            // Want to move RDX into MSR but wrmsr takes EDX:EAX i.e. EDX
            // goes to high 32 bits of MSR, and EAX goes to low order bits
            // <https://www.felixcloutier.com/x86/wrmsr>
            "mov eax, edx",
            "shr rdx, 32", // Shift high bits into EDX
            "wrmsr",
            in("rcx") MSR_KERNEL_GS_BASE,
            in("rdx") crate::gdt::tss_address()
        );
    }
}

// Saves all registers to stack
macro_rules! wrap {
    ($func: ident => $w:ident) => {
        #[naked]
        /// # Safety
        ///
        /// Just dont call the function directly thanks
        unsafe extern "sysv64" fn $w() {
            asm!(
                // Disable interrupts
                "cli",

                "swapgs",
                "mov gs:{tss_temp}, rsp",
                "mov rsp, gs:{tss_timer}",
                "sub rsp, {ks_offset}",

                "sub rsp, 8",
                "push gs:{tss_temp}",
                "swapgs",

                "push r11", // RFLAGS
                "sub rsp, 8",  // CS
                "push rcx", // RIP

                "push rax",
                "push rbx",
                "push rcx",
                "push rdx",

                "push rdi",
                "push rsi",
                "push rbp",
                "push r8",

                "push r9",
                "push r10",
                "push r11",
                "push r12",

                "push r13",
                "push r14",
                "push r15",

                "mov rdi, rsp",
                // Call the hander function
                "call {handler}",

                "pop r15",
                "pop r14",
                "pop r13",

                "pop r12",
                "pop r11",
                "pop r10",
                "pop r9",

                "pop r8",
                "pop rbp",
                "pop rsi",
                "pop rdi",

                "pop rdx",
                "pop rcx",
                "pop rbx",
                "pop rax",

                "add rsp, 24",
                "pop rsp",

                "sysretq",
                handler = sym $func,
                tss_timer = const(0x24 + crate::gdt::TIMER_INTERRUPT_INDEX * 8),
                tss_temp = const(0x24 + 4 * 8),
                ks_offset = const(1024),
                options(noreturn)
            );
        }
    };
}

wrap!(handle_syscall => wrapped_syscall_handler);

// #[repr(usize)]
// pub enum SyscallNumber {
//     Read = 0,
//     Write = 1,
//     Open = 2,
//     Mmap = 9,
//     Ioctl = 16,
//     Getpid = 39,
//     Fork = 57,
//     Exec = 59,
//     Exit = 60,
// }

type SyscallHandler = fn(regs: &Context) -> isize;

// static SYSCALL_TABLE: [SyscallHandler; 1] = [
//     syscall_read,
//     // syscall_write,
//     // syscall_open,
//     // syscall_mmap,
//     // syscall_ioctl,
//     // syscall_getpid,
//     // syscall_fork,
//     // syscall_exec,
//     // syscall_exit,
// ];

pub const READ: usize = 0;
pub const WRITE: usize = 1;
pub const OPEN: usize = 2;
pub const MMAP: usize = 9;
pub const IOCTL: usize = 16;
pub const GET_PID: usize = 39;
pub const FORK: usize = 57;
pub const EXEC: usize = 59;
pub const EXIT: usize = 60;

// fn handle_syscall(stack_frame: &mut InterruptStackFrame, regs: &mut Context) {
fn handle_syscall(regs: &mut Context) {
    let syscall_id = regs.rax;

    // Table jump code - will enable once syscalls are more fleshed out - we use a switch for now
    // if syscall_id < SYSCALL_TABLE.len() {
    //     let handler = SYSCALL_TABLE[syscall_id];

    //     let result = handler(regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9);
    //     regs.rax = result as usize;
    // } else {
    //     regs.rax = -1isize as usize;
    // }

    let result = match syscall_id {
        READ => syscall_read(regs),
        WRITE => syscall_write(regs),
        OPEN => syscall_open(regs),
        MMAP => syscall_mmap(regs),
        IOCTL => syscall_ioctl(regs),
        GET_PID => syscall_getpid(regs),
        FORK => syscall_fork(regs),
        EXEC => syscall_exec(regs),
        EXIT => syscall_exit(regs),
        _ => -1isize as isize,
    };

    regs.rax = result as usize;
}

fn syscall_read(regs: &Context) -> isize {
    let buf: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(regs.rsi as *mut u8, regs.rdx) };

    scheduler::SCHEDULER
        .read()
        .read_file_descriptor(regs.rdi as u32, buf) as isize
}

fn syscall_write(regs: &Context) -> isize {
    let slice: &[u8] =
        unsafe { core::slice::from_raw_parts(VirtAddr::new(regs.rsi as u64).as_ptr(), regs.rdx) };
    if regs.rdi == 1 {
        let string = core::str::from_utf8(slice).unwrap();
        print!("{string}");
    } else {
        scheduler::SCHEDULER
            .read()
            .write_file_descriptor(regs.rdi as u32, slice);
    }
    0
}

fn syscall_open(regs: &Context) -> isize {
    let filename = unsafe { CStr::from_ptr(VirtAddr::new(regs.rdi as u64).as_ptr()) }
        .to_str()
        .unwrap()
        .to_owned();

    let fd = vfs::open(&filename).unwrap();

    scheduler::SCHEDULER.read().add_file_descriptor(fd) as isize
}

fn syscall_mmap(regs: &Context) -> isize {
    // FIX ME - actually map properly, create usermode mapper
    let _fd = regs.r8;

    let memory_info = unsafe { crate::memory::MEMORY_INFO.as_mut().unwrap() };

    let phys_addr =
        crate::memory::translate_addr(VirtAddr::new(0x18000000000), memory_info.phys_mem_offset)
            .unwrap();

    crate::memory::map_physical_address_to_user(VirtAddr::new(0x400000000000), phys_addr, regs.rsi);

    0x400000000000
}

fn syscall_ioctl(regs: &Context) -> isize {
    // FIX ME - expand to actually check arguments rather than just assume we are getting
    // framebuffer info
    scheduler::SCHEDULER
        .read()
        .ioctl(regs.rdi as usize, regs.rsi as u32, regs.rdx as usize);
    // look at return values
    0
}

fn syscall_getpid(_regs: &Context) -> isize {
    scheduler::SCHEDULER.read().get_cur_pid() as isize
}

fn syscall_fork(regs: &Context) -> isize {
    println!(
        "[Kernel] Forking PID: {}",
        scheduler::SCHEDULER.read().get_cur_pid()
    );
    scheduler::SCHEDULER.read().fork_current(regs.clone()) as isize
}

fn syscall_exec(regs: &mut Context) -> isize {
    let filename = unsafe { CStr::from_ptr(VirtAddr::new(regs.rdi as u64).as_ptr()) }
        .to_str()
        .unwrap()
        .to_owned();
    scheduler::SCHEDULER.read().exec(regs, filename) as isize
}

fn syscall_exit(_regs: &mut Context) -> isize {
    // Mark the current process as exiting.
    scheduler::SCHEDULER.read().exit_current();

    // This process must not run anymore. We force a context switch
    // by triggering a Timer interrupt which runs our context switching
    // logic
    unsafe {
        asm!("int 32", options(nomem, nostack));
    }

    unreachable!();
}
