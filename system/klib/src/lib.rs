#![no_std]
#![feature(unsafe_cell_access)]
#![feature(const_trait_impl)]
#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(ptr_metadata)]
#![feature(negative_impls)]

extern crate alloc;

pub mod bootinfo;
pub mod hart;
pub mod kalloc;
pub mod mem;
pub mod panic;

use {
    core::hint,
    log::{debug, info},
    system::hart::HartInfo,
};

#[macro_export]
macro_rules! use_klib {
    ($kmain:ident) => {
        use system::hart::HartInfo;
        /// # Safety
        /// Should be called by the bootloader after it has finished initializing everything
        #[unsafe(no_mangle)]
        unsafe extern "sysv64" fn init() -> ! {
            unsafe {
                // SAFETY: Guarenteed by bootloader
                klib::init($kmain)
            }
        }
    };
}

/// # Safety
/// Should be called once in the BSP by klib
/// Assumes klib is fully functioning
pub type KMainFn = unsafe fn() -> !;

/// # Safety
/// Should be called by the kernel::init as soon as it has been called by the bootloader
pub unsafe fn init(kmain: KMainFn) -> ! {
    let hartinfo = HartInfo::get();

    if !hartinfo.is_bsp {
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
