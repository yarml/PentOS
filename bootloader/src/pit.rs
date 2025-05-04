use core::hint;

use x64::io::Port;

const CH0_DATA: Port<u8> = Port::new(0x40);
const CMD: Port<u8> = Port::new(0x43);

const BASE_FREQ: usize = 1193182;

/// Sleep with a precision of 5us and accuracy > 99% (not counting hardware accuracy) for AT LEAST t us
pub fn sleep_us(t: usize) {
    const TU_PER_5US: usize = 6;
    let us5_amount = (t / 5) + 1;
    let delta_units = us5_amount * TU_PER_5US;
    let expected_delivery_time = 0xFFFF - delta_units;

    // Start timer
    unsafe {
        // # Safety
        // No side effect
        CH0_DATA.write(0xFF);
        CH0_DATA.write(0xFF);
    }
    loop {
        let current_time = unsafe {
            // # Safet
            // No side effect
            CMD.write(0);
            let lo = CH0_DATA.read();
            let hi = CH0_DATA.read();
            lo as u16 | (hi as u16) << 8
        } as usize;

        if current_time < expected_delivery_time {
            break;
        }
        hint::spin_loop();
    }
}
