#[macro_export]
macro_rules! test_print {
    ($($arg:tt)*) => {{
        #[cfg(test)]
        std::println!($($arg)*);
    }};
}
