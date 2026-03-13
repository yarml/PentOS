use {log::Log, system::hart::HartInfo};

pub(crate) fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::STATIC_MAX_LEVEL);
}

static LOGGER: KernelLogger = KernelLogger;

struct KernelLogger;

impl Log for KernelLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let osid = HartInfo::get().osid;

        let mut buf = [0u8; 24]; // "[NNN...] " fits easily
        let prefix = fmt_osid_prefix(&mut buf, osid);

        log_debugcon::log_record_prefixed(record, prefix);
    }

    fn flush(&self) {}
}

fn fmt_osid_prefix(buf: &mut [u8; 24], osid: usize) -> &str {
    let mut pos = 0;

    buf[pos] = b'[';
    pos += 1;

    let mut digits = [0u8; 20];
    let mut n = osid;
    let mut dlen = 0;
    if n == 0 {
        digits[0] = b'0';
        dlen = 1;
    } else {
        while n > 0 {
            digits[dlen] = b'0' + (n % 10) as u8;
            n /= 10;
            dlen += 1;
        }
        digits[..dlen].reverse();
    }

    buf[pos..pos + dlen].copy_from_slice(&digits[..dlen]);
    pos += dlen;

    buf[pos] = b']';
    pos += 1;
    buf[pos] = b' ';
    pos += 1;

    core::str::from_utf8(&buf[..pos]).unwrap()
}