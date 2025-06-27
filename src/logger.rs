use chrono::Local;
use std::fmt;

pub fn info(args: fmt::Arguments) {
    let now = Local::now();
    println!("[{}] INFO {}", now.format("%Y-%m-%d %H:%M:%S"), args);
}

pub fn error(args: fmt::Arguments) {
    let now = Local::now();
    println!("[{}] ERROR {}", now.format("%Y-%m-%d %H:%M:%S"), args);
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::logger::info(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::logger::error(format_args!($($arg)*))
    };
}
