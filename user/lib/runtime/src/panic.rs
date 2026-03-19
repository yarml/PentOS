#![cfg(not(feature = "test"))]

use core::{hint, panic::PanicInfo};

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        hint::spin_loop();
    }
}
