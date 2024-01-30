#![no_std]
#![no_main]

#[macro_use]
extern crate user_api;

#[no_mangle]
fn main() {
    // loop {
    //     let mut x: [u8; 1] = [0; 1];
    //     unsafe {
    //         user_api::syscalls::read(0, &mut x);
    //     };
    //     if x != [0] {
    //         print!("{}", x[0] as char);
    //     }
    // }

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

    println!("[{pid}] This should print twice");

    loop {}
}
