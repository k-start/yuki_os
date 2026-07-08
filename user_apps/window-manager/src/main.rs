#![no_std]
#![no_main]

use event::mouseevent::MOUSE_EVENT;
use user_api::{syscalls::open, window::WindowCommand};
use world::WORLD;

#[macro_use]
extern crate user_api;

extern crate alloc;

mod event;
mod framebuffer;
mod graphics;
mod window;
mod windowmanager;
mod world;

#[no_mangle]
fn main() {
    // Fork into test-binary to test window creation
    // This is just a temporary test
    let mut pid = unsafe { user_api::syscalls::get_pid() };
    println!("[{pid}] Forking...");

    let fork_ret = unsafe { user_api::syscalls::fork() };

    pid = unsafe { user_api::syscalls::get_pid() };

    if fork_ret == 0 {
        println!("[{pid}] Child");
        let _exec_ret = unsafe { user_api::syscalls::exec(b"/initrd/test-binary\0") };
    }

    let wm_controller_fd = unsafe { open(b"/dev/wm_controller\0") };

    loop {
        while let Some(e) = MOUSE_EVENT.lock().poll() {
            WORLD.lock().handle_mouse_event(e);
        }

        const CMD_SIZE: usize = core::mem::size_of::<WindowCommand>();
        let mut request_buf = [0u8; CMD_SIZE];

        let bytes_read = unsafe { user_api::syscalls::read(wm_controller_fd, &mut request_buf) };
        if bytes_read > 0 {
            let cmd: WindowCommand =
                unsafe { core::ptr::read_unaligned(request_buf.as_ptr() as *const WindowCommand) };
            WORLD.lock().handle_ipc(cmd);
        }

        WORLD.lock().render();
    }
}
