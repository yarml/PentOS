use core::arch::asm;
use log::info;
use uefi::Status;
use uefi::entry;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    info!("Hello");
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
