#[macro_export]
macro_rules! gdb_print {
    ($($arg:tt)*) => {{
        #[cfg(test)]
        std::println!($($arg)*);
    }};
}
