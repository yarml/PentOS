use x64::io::Port;
use x64::io::{self};

const PIC_MASTER_PORT_CMD: u16 = 0x20;
const PIC_MASTER_PORT_DATA: u16 = 0x21;

const PIC_SLAVE_PORT_CMD: u16 = 0xA0;
const PIC_SLAVE_PORT_DATA: u16 = 0xA1;

// Intact code from HeliumOS
pub fn disable() {
    let mut master_command = Port::<u8>::new(PIC_MASTER_PORT_CMD);
    let mut master_data = Port::<u8>::new(PIC_MASTER_PORT_DATA);

    let mut slave_command = Port::<u8>::new(PIC_SLAVE_PORT_CMD);
    let mut slave_data = Port::<u8>::new(PIC_SLAVE_PORT_DATA);

    unsafe {
        // # Safety
        // None of these have any side effect on memory

        // Config mode
        master_command.write(0x11);
        slave_command.write(0x11);
        io::wait();

        // Offset master and slave to 0x20 and 0x28 respectively
        master_data.write(0x20);
        slave_data.write(0x28);
        io::wait();

        // Configure master slave relationship
        master_data.write(4);
        slave_data.write(2);
        io::wait();

        // Use 8086 Mode
        master_data.write(1);
        slave_data.write(1);
        io::wait();

        // Mask everything
        master_data.write(0xFF);
        slave_data.write(0xFF);

        io::wait();
    }
}
