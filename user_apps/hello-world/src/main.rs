#![no_std]
#![no_main]

#[macro_use]
extern crate user_api;

#[no_mangle]
fn main() {
    let mut pid = unsafe { user_api::syscalls::get_pid() };
    println!("[{pid}] Forking...");

    let fork_ret = unsafe { user_api::syscalls::fork() };

    pid = unsafe { user_api::syscalls::get_pid() };

    if fork_ret == 0 {
        println!("[{pid}] Child");
        let _exec_ret = unsafe { user_api::syscalls::exec(b"/initrd/test-binary\0") };
    } else {
        println!("[{pid}] Parent");
    }

    println!("[{pid}] This prints once now!");
}
