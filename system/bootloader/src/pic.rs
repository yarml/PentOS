use x64::io::{self, Port};

// Intact code from HeliumOS
pub fn disable() {
    // SAFETY: no other system in the kernel uses these ports
    let mut master_cmd: Port<u8> = unsafe { Port::new(0x20) };
    let mut master_data: Port<u8> = unsafe { Port::new(0x21) };

    let mut slave_cmd: Port<u8> = unsafe { Port::new(0xA0) };
    let mut slave_data: Port<u8> = unsafe { Port::new(0xA1) };

    // Config mode
    master_cmd.write(0x11);
    slave_cmd.write(0x11);
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
