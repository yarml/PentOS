use {core::hint, x64::io::Port};

const CH0_DATA: Port<u8> = Port::new(0x40);
const CMD: Port<u8> = Port::new(0x43);

/// Sleep with a precision of 5us and accuracy > 99% (not counting hardware accuracy) for AT LEAST t us
/// Unless t is too large, in which case it will sleep for the maximum time.
/// The maximum sleep time is 54610us
pub fn sleep_us(t: usize) {
    let t = if t >= 54610 { 54610 } else { t };
    const TU_PER_5US: usize = 6;
    let us5_amount = (t / 5) + 1;
    let delta_units = us5_amount * TU_PER_5US;
    let expected_delivery_time = 0xFFFF - delta_units;

    // Start timer
    unsafe {
        // SAFETY: No side effect on memory
        CH0_DATA.write(0xFF);
        CH0_DATA.write(0xFF);
    }
    let mut last_time = 0xFFFF;
    loop {
        let current_time = unsafe {
            // SAFETY: No side effect on memory
            CMD.write(0);
            let lo = CH0_DATA.read();
            let hi = CH0_DATA.read();
            lo as u16 | (hi as u16) << 8
        } as usize;
        if current_time < expected_delivery_time || last_time > current_time {
            break;
        }
        last_time = current_time;
        hint::spin_loop();
    }
}
