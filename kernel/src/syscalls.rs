use core::arch::asm;

const MSR_STAR: usize = 0xC0000081;

pub fn init() {
    unsafe {
        asm!("mov ecx, 0xC0000080", "rdmsr", "or eax, 1", "wrmsr");

        // write segments to use on syscall/sysret to AMD'S MSR_STAR register
        asm!(
            "xor rax, rax",
            "mov rdx, 0x230008", // use seg selectors 8, 16 for syscall and 43, 51 for sysret
            "wrmsr",
            in("rcx") MSR_STAR);
    }
}
