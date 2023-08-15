pub const WRITE: usize = 1;
pub const EXIT: usize = 60;

pub unsafe fn write(fd: i32, buf: &[u8]) -> isize {
    let r0;
    core::arch::asm!(
        "syscall",
        inlateout("rax") WRITE => r0,
        in("rdi") fd,
        in("rsi") buf.as_ptr(),
        in("rdx") buf.len(),
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags)
    );
    r0
}

pub unsafe fn exit() {
    core::arch::asm!(
        "syscall",
        in("rax") EXIT,
        in("rdi") 0,
        options(nostack, preserves_flags)
    );
}
