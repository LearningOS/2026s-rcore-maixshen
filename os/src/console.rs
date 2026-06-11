//! SBI console driver, for text output
use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

/// Print! to the host console using the format string and arguments.
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?))
    }
}

/// Println! to the host console using the format string and arguments.
#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}
// ==========================================================================
/// Print Trace Log with Grey(90) color
#[macro_export]
macro_rules! trace {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(
            concat!("\x1b[90m[DEBUG] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?
        ))
    }
}

/// Print Debug log with Green(32) color
#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(
            concat!("\x1b[32m[DEBUG] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?
        ))
    }
}

/// Print Info log with Blue(34) color
#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(
            concat!("\x1b[34m[INFO] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?
        ))
    }
}

/// Print Warn Log with Yellow(93) color
#[macro_export]
macro_rules! warn {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(
            concat!("\x1b[93m[INFO] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?
        ))
    }
}

/// Print Error log with Red(31) color
#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(
            concat!("\x1b[31m[ERROR] ", $fmt, "\x1b[0m\n") $(, $($arg)+)?
        ))
    }
}
