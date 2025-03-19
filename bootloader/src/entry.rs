use crate::logger;
use crate::mmap::MemoryMap;
use common::mem::MemoryRegion;
use common::mem::MemorySize;
use common::mem::addr::PhysAddr;
use core::arch::asm;
use log::info;
use uefi::Status;
use uefi::boot;
use uefi::boot::MemoryType;
use uefi::entry;
use uefi::mem::memory_map::MemoryMap as UefiMemoryMap;
use uefi::system;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    system::with_stdout(|stdout| {
        // If it fails, we don't really care.
        stdout.clear().ok();
    });
    unsafe {
        // SAFETY: We call logger::diable() before exiting UEFI boot services.
        logger::init();
    }

    logger::disable();

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
