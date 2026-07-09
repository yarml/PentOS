use {
    colored::Colorize,
    std::{
        fmt::Display,
        process,
        sync::atomic::{AtomicUsize, Ordering},
    },
};

pub struct Status;

static LEVEL: AtomicUsize = AtomicUsize::new(0);

impl Status {
    pub fn doing(verb: &str, message: impl Display) {
        let level = LEVEL.load(Ordering::Relaxed);
        eprintln!(
            "{}{:12} {}",
            "  ".repeat(level),
            verb.green().bold(),
            message
        );
    }
    pub fn push(verb: &str, message: impl Display) {
        let level = LEVEL.fetch_add(1, Ordering::Relaxed);
        eprintln!(
            "{}{:12} {}",
            "  ".repeat(level),
            verb.green().bold(),
            message
        );
    }

    pub fn pop() {
        LEVEL.fetch_sub(1, Ordering::Relaxed);
    }
    pub fn warning(message: impl Display) {
        eprintln!("{:>12} {}", "warning:".yellow().bold(), message);
    }
    pub fn error(message: impl Display) -> ! {
        eprintln!("{:>12} {}", "error:".red().bold(), message);
        process::exit(1)
    }

    pub fn indent(message: impl Display) -> String {
        let level = LEVEL.load(Ordering::Relaxed);
        format!("{}{}", "  ".repeat(level), message)
    }
}
