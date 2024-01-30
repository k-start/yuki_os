pub const READ: usize = 0;
pub const WRITE: usize = 1;
pub const OPEN: usize = 2;
pub const GET_PID: usize = 39;
pub const FORK: usize = 57;
pub const EXEC: usize = 59;
pub const EXIT: usize = 60;

pub unsafe fn read(fd: i32, buf: &mut [u8]) {
    core::arch::asm!(
        "syscall",
        in("rax") READ,
        in("rdi") fd,
        in("rsi") buf.as_ptr(),
        in("rdx") buf.len(),
        options(nostack, preserves_flags)
    );
}

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

pub unsafe fn open(filename: &[u8]) {
    core::arch::asm!(
        "syscall",
        in("rax") OPEN,
        in("rdi") filename.as_ptr(), // Filename pointer
        in("rsi") 0, // Flags
        in("rdx") 0, // mode
        options(nostack, preserves_flags)
    );
}

pub unsafe fn get_pid() -> isize {
    let r0;
    core::arch::asm!(
        "syscall",
        inlateout("rax") GET_PID => r0,
        options(nostack, preserves_flags)
    );
    r0
}

pub unsafe fn fork() -> isize {
    let r0;
    core::arch::asm!(
        "syscall",
        inlateout("rax") FORK => r0,
        options(nostack, preserves_flags)
    );
    r0
}

// FIX ME - implement args
pub unsafe fn exec(filename: &[u8]) -> isize {
    let r0;
    core::arch::asm!(
        "syscall",
        in("rdi") filename.as_ptr(),
        inlateout("rax") EXEC => r0,
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
