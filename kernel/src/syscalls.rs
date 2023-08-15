use core::arch::asm;

use x86_64::{structures::idt::InterruptStackFrame, VirtAddr};

use crate::scheduler;

const MSR_STAR: usize = 0xc0000081;
const MSR_LSTAR: usize = 0xc0000082;
const MSR_FMASK: usize = 0xc0000084;

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
    }
}

// Saves all registers to stack
macro_rules! wrap {
    ($fn: ident => $w:ident) => {
        #[naked]
        /// # Safety
        ///
        /// Just dont call the function directly thanks
        unsafe extern "sysv64" fn $w() {
            asm!(
                "push rax",
                "push rcx",
                "push rdx",
                "push rsi",
                "push rdi",
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "mov rsi, rsp", // Arg #2: register list
                "mov rdi, rsp", // Arg #1: interupt frame
                "add rdi, 9 * 8",
                "call {}",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rdi",
                "pop rsi",
                "pop rdx",
                "pop rcx",
                "pop rax",
                "sysretq",
                sym $fn,
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

pub const WRITE: usize = 1;
pub const EXIT: usize = 60;

fn handle_syscall(_stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
    match regs.rax {
        WRITE => unsafe {
            let slice: &[u8] =
                core::slice::from_raw_parts(VirtAddr::new(regs.rsi as u64).as_ptr(), regs.rdx);

            let string = core::str::from_utf8(slice).unwrap();
            print!("{string}");
        },
        EXIT => {
            scheduler::SCHEDULER.exit_current();
        }
        _ => {}
    }
    regs.rax = 0;
}
