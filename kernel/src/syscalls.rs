use core::{arch::asm, ffi::CStr};

use alloc::borrow::ToOwned;
use x86_64::VirtAddr;

use crate::{process::Context, scheduler};

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

#[repr(align(8), C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Registers {
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,
}

pub const READ: usize = 0;
pub const WRITE: usize = 1;
pub const OPEN: usize = 2;
pub const IOCTL: usize = 16;
pub const GET_PID: usize = 39;
pub const FORK: usize = 57;
pub const EXEC: usize = 59;
pub const EXIT: usize = 60;

// fn handle_syscall(stack_frame: &mut InterruptStackFrame, regs: &mut Context) {
fn handle_syscall(regs: &mut Context) {
    // println!("{:?}", regs);

    match regs.rax {
        READ => {
            let buf: &mut [u8] =
                unsafe { core::slice::from_raw_parts_mut(regs.rsi as *mut u8, regs.rdx) };
            scheduler::SCHEDULER
                .read()
                .read_file_descriptor(regs.rdi as u32, buf);
            regs.rax = 0;
        }
        WRITE => unsafe {
            let slice: &[u8] =
                core::slice::from_raw_parts(VirtAddr::new(regs.rsi as u64).as_ptr(), regs.rdx);
            if regs.rdi == 1 {
                let string = core::str::from_utf8(slice).unwrap();
                print!("{string}");
            } else {
                scheduler::SCHEDULER
                    .read()
                    .write_file_descriptor(regs.rdi as u32, slice);
            }
            regs.rax = 0;
        },
        OPEN => {
            let filename = unsafe { CStr::from_ptr(VirtAddr::new(regs.rdi as u64).as_ptr()) }
                .to_str()
                .unwrap()
                .to_owned();

            let fd = crate::fs::vfs::open(&filename).unwrap();

            regs.rax = scheduler::SCHEDULER.read().add_file_descriptor(&fd);

            // println!("open {filename}");
        }
        IOCTL => {
            scheduler::SCHEDULER.read().ioctl(
                regs.rdi as usize,
                regs.rsi as u32,
                regs.rdx as usize,
            );
            // println!("{:?}", regs.rdx as u64);
        }
        GET_PID => {
            regs.rax = scheduler::SCHEDULER.read().get_cur_pid();
        }
        FORK => {
            println!(
                "[Kernel] Forking PID: {}",
                scheduler::SCHEDULER.read().get_cur_pid()
            );
            regs.rax = scheduler::SCHEDULER.read().fork_current(regs.clone());
        }
        EXEC => {
            let filename = unsafe { CStr::from_ptr(VirtAddr::new(regs.rdi as u64).as_ptr()) }
                .to_str()
                .unwrap()
                .to_owned();
            regs.rax = scheduler::SCHEDULER.read().exec(regs, filename);
        }
        EXIT => {
            scheduler::SCHEDULER.read().exit_current();
        }
        _ => {}
    }
}
