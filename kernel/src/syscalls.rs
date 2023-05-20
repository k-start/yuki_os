use core::arch::asm;

use alloc::vec::Vec;

const MSR_STAR: usize = 0xc0000081;
const MSR_LSTAR: usize = 0xc0000082;
const MSR_FMASK: usize = 0xc0000084;

pub fn init() {
    let handler_addr = handle_syscall as *const () as u64;

    unsafe {
        asm!("mov ecx, 0xC0000080", "rdmsr", "or eax, 1", "wrmsr");

        // write segments to use on syscall/sysret to AMD'S MSR_STAR register
        // asm!(
        //     "xor rax, rax",
        //     "mov rdx, 0x230008", // use seg selectors 8, 16 for syscall and 43, 51 for sysret
        //     "wrmsr",
        //     in("rcx") MSR_STAR);

        asm!("\
        xor rdx, rdx
        mov rax, 0x200
        wrmsr", in("rcx") MSR_FMASK, out("rdx") _);
        // write handler address to AMD's MSR_LSTAR register
        asm!("\
        mov rdx, rax
        shr rdx, 32
        wrmsr", in("rax") handler_addr, in("rcx") MSR_LSTAR, out("rdx") _);
        // write segments to use on syscall/sysret to AMD'S MSR_STAR register
        asm!("\
        xor rax, rax
        mov rdx, 0x230008 // use seg selectors 8, 16 for syscall and 43, 51 for sysret
        wrmsr", in("rcx") MSR_STAR, out("rax") _, out("rdx") _);
    }
}

fn handle_syscall() {
    unsafe {
        asm!(
            "\
        push rcx // backup registers for sysretq
        push r11
        push rbp // save callee-saved registers
        push rbx
        push r12
        push r13
        push r14
        push r15
        mov rbp, rsp // save rsp
        sub rsp, 0x400 // make some room in the stack
        push rax // backup syscall params while we get some stack space
        push rdi
        push rsi
        push rdx
        push r10"
        );
    }
    let syscall_stack: Vec<u8> = Vec::with_capacity(0x10000);
    let stack_ptr = syscall_stack.as_ptr();
    unsafe {
        asm!(
            "\
        pop r10 // restore syscall params to their registers
        pop rdx
        pop rsi
        pop rdi
        pop rax
        mov rsp, rbx // move our stack to the newly allocated one
        sti // enable interrupts"
        );
        // inout("rbx") stack_ptr => _);
    }
    let syscall: u64;
    let arg0: u64;
    let arg1: u64;
    let arg2: u64;
    let arg3: u64;
    unsafe {
        // move the syscall arguments from registers to variables
        asm!("nop",
        out("rax") syscall, out("rdi") arg0, out("rsi") arg1, out("rdx") arg2, out("r10") arg3);
    }
    // println!("Syscall");
    let retval: i64 = 0;
    unsafe {
        asm!("\
        mov rbx, {} // save return value into rbx so that it's maintained through free
        cli",
        in(reg) retval // disable interrupts while restoring the stack
        );
    }
    drop(syscall_stack); // we can now drop the syscall temp stack
    unsafe {
        asm!(
            "\
        mov rax, rbx // restore syscall return value from rbx to rax
        mov rsp, rbp // restore rsp from rbp
        pop r15 // restore callee-saved registers
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp // restore stack and registers for sysretq
        pop r11
        pop rcx
        sysretq // back to userland",
            options(noreturn)
        );
    }
}
