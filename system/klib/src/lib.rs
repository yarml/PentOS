#![no_std]
#![feature(unsafe_cell_access)]
#![feature(const_trait_impl)]
#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(ptr_metadata)]
#![feature(negative_impls)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod bootinfo;
pub mod dev;
pub mod hart;
pub mod sync;
pub mod task;

mod interrupts;
mod kalloc;
mod logger;
mod mem;
mod panic;

use {
    core::{
        hint,
        sync::atomic::{AtomicBool, Ordering},
    },
    log::{debug, info},
    system::hart::HartInfo,
};

#[macro_export]
macro_rules! use_klib {
    ($kmain:ident) => {
        extern crate alloc;
        /// # Safety
        /// Should be called by the bootloader after it has finished initializing everything
        #[unsafe(no_mangle)]
        unsafe extern "sysv64" fn init() -> ! {
            unsafe {
                // SAFETY: Guarenteed by bootloader
                klib::init($kmain())
            }
        }
    };
}

/// # Safety
/// Should be called by the kernel::init as soon as it has been called by the bootloader
pub unsafe fn init(kmain: impl Future<Output = ()> + Send + 'static) -> ! {
    static BSP_SETUP: AtomicBool = AtomicBool::new(false);

    let hartinfo = HartInfo::get();

    if !hartinfo.is_bsp() {
        while !BSP_SETUP.load(Ordering::Acquire) {
            hint::spin_loop();
        }
        common_setup();
        task::run();
    }

    logger::init();
    info!("KLib initialization...");

    unsafe {
        // SAFETY: Called once in the BSP and no other allocator can be called before this initialization ends
        mem::phys::init()
    };

    interrupts::setup();

    dev::init();

    task::init();
    debug!("Spawning kmain");
    task::spawn(kmain);

    BSP_SETUP.store(true, Ordering::Release);

    common_setup();
    task::run()
}

fn common_setup() {
    interrupts::load();
    interrupts::enable();
}

#[cfg(feature = "test")]
pub mod mem_test {
    pub use super::mem::*;
}
