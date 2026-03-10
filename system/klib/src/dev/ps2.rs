mod events;
mod state_machine;

use {
    log::{debug, warn},
    spinlocks::mutex::Mutex,
    x64::{
        interrupts,
        io::{self, Port},
    },
};

const CMD_READ_CONFIG: u8 = 0x20;
const CMD_WRITE_CONFIG: u8 = 0x60;

const KB_CMD_SET_SCAN_CODE_SET: u8 = 0xF0;
const KB_RESPONSE_ACK: u8 = 0xFA;
const KB_RESPONSE_RESEND: u8 = 0xFE;

const CONFIG_TRANSLATION_BIT: u8 = 1 << 6;
const CONFIG_IRQ1_BIT: u8 = 1 << 0;

static DATA_PORT: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x60) });
static CMD_PORT: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x64) });

pub(crate) fn init() {
    let mut cmd_port = CMD_PORT.lock();
    let mut data_port = DATA_PORT.lock();

    write_cmd(&mut cmd_port, CMD_READ_CONFIG);
    let mut config = read_data(&mut cmd_port, &mut data_port);
    config &= !CONFIG_TRANSLATION_BIT;
    config |= CONFIG_IRQ1_BIT;
    write_cmd(&mut cmd_port, CMD_WRITE_CONFIG);
    write_data_(&mut cmd_port, &mut data_port, config);

    send_kbd(&mut cmd_port, &mut data_port, KB_CMD_SET_SCAN_CODE_SET);
    send_kbd(&mut cmd_port, &mut data_port, 0x02);
}

pub(crate) fn on_key_event() {
    let scancode = {
        interrupts::with_disabled(|| {
            let mut cmd_port = CMD_PORT.lock();
            let mut data_port = DATA_PORT.lock();
            read_data(&mut cmd_port, &mut data_port)
        })
    };

    debug!("Scancode: {:#x}", scancode);
}

fn write_cmd(cmd_port: &mut Port<u8>, cmd: u8) {
    wait_write_ready(cmd_port);
    cmd_port.write(cmd)
}

fn send_kbd(cmd_port: &mut Port<u8>, data_port: &mut Port<u8>, kb_command: u8) {
    loop {
        write_data_(cmd_port, data_port, kb_command);
        let resp = read_data(cmd_port, data_port);
        if resp == KB_RESPONSE_ACK {
            return;
        }
        if resp == KB_RESPONSE_RESEND {
            continue;
        }
        warn!("unexpected keyboard response: {resp:#x}");
        io::wait();
    }
}

fn wait_write_ready(cmd_port: &mut Port<u8>) {
    while cmd_port.read() & 0x2 != 0 {
        io::wait();
    }
}

fn wait_read_ready(cmd_port: &mut Port<u8>) {
    while cmd_port.read() & 0x1 == 0 {
        io::wait();
    }
}

fn read_data(cmd_port: &mut Port<u8>, data_port: &mut Port<u8>) -> u8 {
    wait_read_ready(cmd_port);
    data_port.read()
}

fn write_data_(cmd_port: &mut Port<u8>, data_port: &mut Port<u8>, data: u8) {
    wait_write_ready(cmd_port);
    data_port.write(data)
}
