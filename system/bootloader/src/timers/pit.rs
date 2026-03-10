use {core::hint, spinlocks::mutex::Mutex, x64::io::Port};

static CH0_DATA: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x40) });
static CMD: Mutex<Port<u8>> = Mutex::new(unsafe { Port::new(0x43) });

/// Sleep with a precision of 5us and accuracy > 99% (not counting hardware accuracy) for AT LEAST t us
/// Unless t is too large, in which case it will sleep for the maximum time.
/// The maximum sleep time is 54610us
///
/// # Note
/// If called from more than one hart at a time, the sleep time will be very innacurate for the second hart
pub fn sleep_us(t: usize) {
    let t = if t >= 54610 { 54610 } else { t };
    const TU_PER_5US: usize = 6;
    let us5_amount = (t / 5) + 1;
    let delta_units = us5_amount * TU_PER_5US;
    let expected_delivery_time = 0xFFFF - delta_units;

    let mut ch0_data = CH0_DATA.lock();
    let mut cmd = CMD.lock();

    // Start timer
    ch0_data.write(0xFF);
    ch0_data.write(0xFF);

    let mut last_time = 0xFFFF;
    loop {
        let current_time = {
            cmd.write(0);
            let lo = ch0_data.read();
            let hi = ch0_data.read();
            lo as u16 | (hi as u16) << 8
        } as usize;
        if current_time < expected_delivery_time || last_time > current_time {
            break;
        }
        last_time = current_time;
        hint::spin_loop();
    }
}
