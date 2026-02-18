#![no_std]
#![no_main]

extern crate alloc;

mod init;

use {alloc::boxed::Box, core::hint, log::debug};

/// # Safety
/// Should be called once in the BSP by klib
/// Assumes klib is fully functioning
unsafe fn kmain() -> ! {
    let a = Box::new(10);
    debug!("a: {a}");

    loop {
        hint::spin_loop();
    }
}
