#![no_std]
#![no_main]

use core::hint;

mod init;
mod panic;

/// # Safety
/// Should be called once in the BSP by klib
/// Assumes klib is fully functioning
unsafe fn kmain() -> ! {
    loop {
        hint::spin_loop();
    }
}
