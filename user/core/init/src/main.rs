#![no_std]
#![no_main]

use core::hint;

extern crate runtime;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    loop {
        hint::spin_loop();
    }
}
