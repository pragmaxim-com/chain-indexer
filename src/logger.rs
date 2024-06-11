use chrono::Local;
use std::fmt;

pub fn log(args: fmt::Arguments) {
    let now = Local::now();
    println!("[{}] {}", now.format("%Y-%m-%d %H:%M:%S"), args);
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::logger::log(format_args!($($arg)*))
    };
}
