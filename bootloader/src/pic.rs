use x64::io::Port;
use x64::io::{self};

const PIC_MASTER_CMD: Port<u8> = Port::new(0x20);
const PIC_MASTER_DATA: Port<u8> = Port::new(0x21);

const PIC_SLAVE_CMD: Port<u8> = Port::new(0xA0);
const PIC_SLAVE_DATA: Port<u8> = Port::new(0xA1);

// Intact code from HeliumOS
pub fn disable() {
    unsafe {
        // # Safety
        // None of these have any side effect on memory

        // Config mode
        PIC_MASTER_CMD.write(0x11);
        PIC_SLAVE_CMD.write(0x11);
        io::wait();

        // Offset master and slave to 0x20 and 0x28 respectively
        PIC_MASTER_DATA.write(0x20);
        PIC_SLAVE_DATA.write(0x28);
        io::wait();

        // Configure master slave relationship
        PIC_MASTER_DATA.write(4);
        PIC_SLAVE_DATA.write(2);
        io::wait();

        // Use 8086 Mode
        PIC_MASTER_DATA.write(1);
        PIC_SLAVE_DATA.write(1);
        io::wait();

        // Mask everything
        PIC_MASTER_DATA.write(0xFF);
        PIC_SLAVE_DATA.write(0xFF);

        io::wait();
    }
}
