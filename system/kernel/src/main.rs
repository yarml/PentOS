#![no_std]
#![no_main]

use core::hint;

klib::use_klib!(kmain);

/// # Safety
/// Should be called once in the BSP by klib
/// Assumes klib is fully functioning
unsafe fn kmain() -> ! {
    loop {
        hint::spin_loop();
    }
}
