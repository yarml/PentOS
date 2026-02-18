#![no_std]
#![feature(unsafe_cell_access)]
#![feature(const_trait_impl)]
#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(ptr_metadata)]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod bootinfo;
pub mod kalloc;
pub mod mem;
pub mod panic;

use {core::hint, log::info};

/// # Safety
/// Should be called once in the BSP by klib
/// Assumes klib is fully functioning
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
        // SAFETY: Called once in the BSP and no other allocator can be called before this initialization ends
        mem::phys::init()
    };

    unsafe {
        // SAFETY: klib initialized
        kmain()
    }
}
