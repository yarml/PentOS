use core::hint;

use {acpi::table::PmTimerInfo, x64::io::Port};

const PM_TIMER_FREQ_HZ: usize = 3_579_545;

/// Sleep for AT LEAST `t` microseconds using the ACPI PM timer.
///
/// This is safe to call from multiple harts simultaneously — the PM timer
/// is read-only and requires no reprogramming.
///
/// There is no upper bound on `t`, unlike the PIT implementation, since
/// the PM timer wraps every ~4.7 seconds (24-bit) or ~1200 seconds (32-bit)
/// and we handle wraps correctly. Very large values of `t` will simply spin
/// for a long time.
///
/// # Safety
/// `info.port` must be the valid I/O port of the ACPI PM timer.
/// And must be called from one hart at a time.
pub unsafe fn sleep_us(t: usize, info: &PmTimerInfo) {
    let mut port = unsafe { Port::<u32>::new(info.port) };
    let mask: u32 = if info.is_32bit {
        0xFFFF_FFFF
    } else {
        0x00FF_FFFF
    };

    let mut read_pm = || {
        port.read() & mask
    };
    let ticks_needed = ((t * PM_TIMER_FREQ_HZ) / 1_000_000) + 1;

    let start = read_pm();

    loop {
        hint::spin_loop();
        let now = read_pm();
        let elapsed = now.wrapping_sub(start) & mask;
        if elapsed as usize >= ticks_needed {
            break;
        }
    }
}
