#![no_std]
#![feature(unsafe_cell_access)]
#![feature(const_trait_impl)]

use {boot_protocol::BootInfo, config::vmem::BOOTINFO_REGION, core::hint, x64::mem::addr::Address};

#[cfg(test)]
extern crate alloc;
#[cfg(test)]
extern crate std;

use log::info;

pub type KMainFn = unsafe fn() -> !;

/// # Safety
/// Should be called by the kernel::init as soon as it has been called by the bootloader
pub unsafe fn init(is_bsp: bool, kmain: KMainFn) -> ! {
    if !is_bsp {
        loop {
            hint::spin_loop();
        }
    }
    log_debugcon::init();
    info!("Kernel library initialization");

    unsafe {
        // SAFETY: klib initialized
        kmain()
    }
}
