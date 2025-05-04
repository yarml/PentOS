use x64::io::Port;
use x64::io::{self};

const MASTER_CMD: Port<u8> = Port::new(0x20);
const MASTER_DATA: Port<u8> = Port::new(0x21);

const SLAVE_CMD: Port<u8> = Port::new(0xA0);
const SLAVE_DATA: Port<u8> = Port::new(0xA1);

// Intact code from HeliumOS
pub fn disable() {
    unsafe {
        // # Safety
        // None of these have any side effect on memory

        // Config mode
        MASTER_CMD.write(0x11);
        SLAVE_CMD.write(0x11);
        io::wait();

        // Offset master and slave to 0x20 and 0x28 respectively
        MASTER_DATA.write(0x20);
        SLAVE_DATA.write(0x28);
        io::wait();

        // Configure master slave relationship
        MASTER_DATA.write(4);
        SLAVE_DATA.write(2);
        io::wait();

        // Use 8086 Mode
        MASTER_DATA.write(1);
        SLAVE_DATA.write(1);
        io::wait();

        // Mask everything
        MASTER_DATA.write(0xFF);
        SLAVE_DATA.write(0xFF);

        io::wait();
    }
}
