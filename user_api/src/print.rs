use core::fmt::{self, Write};

use crate::syscalls;

pub struct Writer {}

impl Writer {
    pub fn new() -> Writer {
        Writer {}
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            syscalls::write(1, s.as_bytes());
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
#[inline(always)]
pub fn _print(args: fmt::Arguments) {
    Writer::new().write_fmt(args).unwrap();
}
