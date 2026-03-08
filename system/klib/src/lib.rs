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
pub mod hart;
pub mod interrupts;
pub mod task;

mod kalloc;
mod mem;
mod panic;

use {
    crate::{
        interrupts::get_timestamp,
        task::{Executor, Task},
    },
    core::{
        hint,
        pin::Pin,
        sync::atomic::{AtomicBool, Ordering},
        task::{Context, Poll},
    },
    log::{debug, info},
    system::hart::HartInfo,
};

#[macro_export]
macro_rules! use_klib {
    ($kmain:ident) => {
        extern crate alloc;
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
    static BSP_SETUP: AtomicBool = AtomicBool::new(false);

    let hartinfo = HartInfo::get();

    if !hartinfo.is_bsp() {
        while !BSP_SETUP.load(Ordering::Relaxed) {
            hint::spin_loop();
        }
        common_setup();
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

    interrupts::setup();

    BSP_SETUP.store(true, Ordering::Relaxed);

    common_setup();

    let mut executor = Executor::new();

    executor.spawn(Task::new(task1()));
    executor.spawn(Task::new(task2()));

    executor.run();

    loop {
        hint::spin_loop();
    }
    unsafe {
        // SAFETY: klib initialized
        kmain()
    }
}

fn common_setup() {
    interrupts::load();
    interrupts::enable();
}

#[cfg(feature = "test")]
pub mod mem_test {
    pub use super::mem::*;
}

async fn task1() {
    loop {
        debug!("Task 1");
        sleep(1).await;
    }
}

async fn task2() {
    loop {
        debug!("Task 2");
        sleep(5).await;
    }
}

fn sleep(seconds: usize) -> Sleeper {
    let current = get_timestamp();
    let end = current + seconds * 100;

    Sleeper { end_time: end }
}

struct Sleeper {
    end_time: usize,
}

impl Future for Sleeper {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let current = get_timestamp();
        if current > self.end_time {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
