#![cfg(not(test))]

use {
    core::{arch::asm, panic::PanicInfo},
    log::error,
};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log_debugcon::master_unlock();
    if let Some(location) = info.location() {
        error!(
            "Kernel Panic({location}): {message}",
            message = info.message()
        );
    } else {
        error!("Kernel Panic: {message}", message = info.message());
    }
    loop {
        unsafe { asm!("hlt") }
    }
}
