use {
    core::{
        arch::asm,
        fmt::{self, Write},
        hint,
        sync::atomic::{AtomicBool, Ordering},
    },
    log::Log,
};

pub fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::STATIC_MAX_LEVEL);
}

static LOGGER: Logger = Logger {
    lock: AtomicBool::new(false),
};

pub struct Logger {
    lock: AtomicBool,
}

struct LogEntry<'w, W: Write> {
    writer: &'w mut W,
    level: log::Level,
    in_transaction: bool,
}

#[cfg(feature = "log-debugcon")]
struct DebugConWriter;

#[cfg(feature = "log-debugcon")]
impl DebugConWriter {
    const IO_PORT: u16 = 0xE9;
}

#[cfg(feature = "log-debugcon")]
impl Write for DebugConWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            unsafe { asm!("out dx, al", in("dx") Self::IO_PORT, in("al") byte) };
        }

        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        for byte in (c as u32).to_ne_bytes() {
            unsafe { asm!("out dx, al", in("dx") Self::IO_PORT, in("al") byte) };
        }

        Ok(())
    }
}

impl<W: Write> Write for LogEntry<'_, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if !self.in_transaction {
            write!(self.writer, "\x08\x08\x08\x08\x08\x08\x08\x08[{:>5}] ", self.level)?;
            self.in_transaction = true;
        }

        let mut lines = s.lines();
        let first_line = lines.next().unwrap();
        write!(self.writer, "{first_line}")?;

        for line in lines {
            write!(self.writer, "\n        {line}")?;
        }

        if s.ends_with('\n') {
            write!(self.writer, "\n        ")?;
        }

        Ok(())
    }

    
}

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        while self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }

        #[cfg(feature = "log-debugcon")]
        {
            let mut entry = LogEntry {
                writer: &mut DebugConWriter,
                level: record.level(),
                in_transaction: false,
            };

            let _ = writeln!(entry, "{}", *record.args());
        }

        self.lock.store(false, Ordering::Release);
    }

    fn flush(&self) {
        // NOOP
    }
}
