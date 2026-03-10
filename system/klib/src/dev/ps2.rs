use {
    core::{
        hint,
        sync::atomic::{AtomicBool, Ordering},
    },
    log::debug,
    x64::{interrupts, io::Port},
};

const DATA_PORT: Port<u8> = Port::new(0x60);
const CMD_PORT: Port<u8> = Port::new(0x64);

const CMD_READ_CONFIG: u8 = 0x20;
const CMD_WRITE_CONFIG: u8 = 0x60;

const KB_CMD_SET_SCAN_CODE_SET: u8 = 0xF0;
const KB_RESPONSE_ACK: u8 = 0xFA;

const CONFIG_TRANSLATION_BIT: u8 = 1 << 6;
const CONFIG_IRQ1_BIT: u8 = 1 << 0;

pub(crate) fn init() {
    interrupts::with_disabled(|| {
        write_cmd(CMD_READ_CONFIG);
        let mut config = read_data();
        config &= !CONFIG_TRANSLATION_BIT;
        config |= CONFIG_IRQ1_BIT;
        write_cmd(CMD_WRITE_CONFIG);
        write_data(config);

        write_data(KB_CMD_SET_SCAN_CODE_SET);
        let ack = read_data();
        assert_eq!(ack, KB_RESPONSE_ACK);
        write_data(0x02);
        let ack = read_data();
        assert_eq!(ack, KB_RESPONSE_ACK);

        // read_data();
    });
}

pub(crate) fn on_key_event() {
    static BUSY: AtomicBool = AtomicBool::new(false);

    while BUSY
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        hint::spin_loop();
    }

    interrupts::with_disabled(|| {
        let scancode = read_data();
        debug!("Scancode: {:#x}", scancode);
    });

    BUSY.store(false, Ordering::Release);
}

fn wait_write_ready() {
    while unsafe { CMD_PORT.read() } & 0x2 != 0 {
        hint::spin_loop();
    }
}

fn wait_read_ready() {
    while unsafe { CMD_PORT.read() } & 0x1 == 0 {
        hint::spin_loop();
    }
}

fn read_data() -> u8 {
    wait_read_ready();
    unsafe { DATA_PORT.read() }
}

fn write_data(data: u8) {
    wait_write_ready();
    unsafe { DATA_PORT.write(data) }
}

fn write_cmd(cmd: u8) {
    wait_write_ready();
    unsafe { CMD_PORT.write(cmd) }
}
