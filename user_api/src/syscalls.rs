pub const READ: usize = 0;
pub const WRITE: usize = 1;
pub const OPEN: usize = 2;
pub const MMAP: usize = 9;
pub const IOCTL: usize = 16;
pub const GET_PID: usize = 39;
pub const FORK: usize = 57;
pub const EXEC: usize = 59;
pub const EXIT: usize = 60;

pub unsafe fn read(fd: usize, buf: &mut [u8]) -> isize {
    let r0;
    core::arch::asm!(
        "syscall",
        inlateout("rax") READ => r0,
        in("rdi") fd,
        in("rsi") buf.as_ptr(),
        in("rdx") buf.len(),
        options(nostack, preserves_flags)
    );
    r0
}

pub unsafe fn write(fd: usize, buf: &[u8]) -> isize {
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

pub unsafe fn open(filename: &[u8]) -> usize {
    let r0;
    core::arch::asm!(
        "syscall",
        inlateout("rax") OPEN => r0,
        in("rdi") filename.as_ptr(), // Filename pointer
        in("rsi") 0, // Flags
        in("rdx") 0, // mode
        options(nostack, preserves_flags)
    );
    r0
}

pub unsafe fn mmap(ptr: usize, len: usize, fd: usize) -> usize {
    let r0;
    core::arch::asm!(
        "syscall",
        inlateout("rax") MMAP => r0,
        in("rdi") ptr, // pointer
        in("rsi") len, // length
        in("r8") fd, // file descriptor
        options(nostack, preserves_flags)
    );
    r0
}

pub unsafe fn ioctl(fd: usize, cmd: u32, arg: usize) {
    core::arch::asm!(
        "syscall",
        in("rax") IOCTL,
        in("rdi") fd, // File descriptor
        in("rsi") cmd, // Command
        in("rdx") arg, // Argument
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
