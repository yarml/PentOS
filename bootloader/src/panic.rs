use core::arch::asm;
use core::panic::PanicInfo;
use log::error;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "Bootloader Panic({location}): {message}",
            message = info.message()
        );
    } else {
        error!("Bootloader Panic: {message}", message = info.message());
    }
    loop {
        unsafe { asm!("hlt") }
    }
}
