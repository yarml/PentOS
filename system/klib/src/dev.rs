pub mod framebuffer;
pub mod pci;
pub mod ps2;
pub mod timer;

use core::hint::black_box;

pub use klib_macros::driver;
use log::info;

pub struct Driver {
    pub init: fn(),
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

unsafe extern "C" {
    static __driver_start: usize;
    static __driver_end: usize;
}

pub(crate) fn init() {
    info!("Loading drivers");
    load_drivers();

    framebuffer::init();
    ps2::init();
    pci::init();
}

fn load_drivers() {
    drivers().for_each(|driver| {
        (driver.init)();
        info!("Loaded driver {}", driver.name);
    });
}

fn drivers() -> impl Iterator<Item = &'static Driver> {
    gen {
        // First time I encounter this class of optimizer bugs, blackbox is necessary in case there
        // are 0 drivers, __driver_start and __driver_end will both be the same variable with
        // 2 different names, hence have the same address, but Rust assumes two variables are always distinct,
        // therefore, it will assume the loop will always at least have one iteration, and
        // still iterate and attempt to initialize a first driver, even when none are present
        let driver_start = black_box(&raw const __driver_start as usize);
        let driver_end = black_box(&raw const __driver_end as usize);

        let mut driver_current = driver_start;
        while driver_current != driver_end {
            let driver = driver_current as *const &Driver;
            let driver = unsafe { *driver };

            yield driver;

            driver_current += 8;
        }
    }
}
