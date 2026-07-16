#![no_std]

use klib::{dev::driver, log::info};

#[driver]
pub fn init() {
    info!(
        "Hello NVMe driver: {}. {:016x}",
        __DRIVER.name, &__DRIVER_PTR as *const _ as usize
    );
}
