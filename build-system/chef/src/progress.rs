use colored::Colorize;

#[macro_export]
macro_rules! print_action {
    ($depth:expr, $action:expr, $($arg:tt)*) => {
        $crate::progress::show_action($depth, $action, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! print_error {
    ($($arg:tt)*) => {
        $crate::progress::show_error(&format!($($arg)*));
    };
}

pub fn show_action(depth: usize, action: &str, description: &str) {
    let depth = depth + 4;
    eprintln!(
        "{space:depth$}{action} {description}",
        space = ' ',
        action = action.green().bold()
    )
}
