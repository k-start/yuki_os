#![no_std]

#[macro_use]
pub mod print;
pub mod syscalls;

use core::panic::PanicInfo;
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("App panic!\n{:?}", info);
    unsafe {
        syscalls::exit();
    };
    loop {}
}

extern "C" {
    fn main() -> ();
}

#[no_mangle]
pub unsafe extern "C" fn _start() {
    init_heap();
    #[cfg(not(test))]
    main();
    syscalls::exit();
    loop {}
}

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
pub fn init_heap() {
    let heap_start = 0x5000_0000_0000;
    let heap_size = 0x100000;
    unsafe {
        ALLOCATOR.lock().init(heap_start, heap_size);
    }
}
