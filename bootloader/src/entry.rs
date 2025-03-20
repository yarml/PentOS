use crate::allocator::PostBootAllocator;
use crate::allocator::PreBootAllocator;
use crate::bootstage;
use crate::kernel;
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
    info!("Booting PentOS...");
    let allocator = PreBootAllocator;

    let kernel = kernel::load_kernel(&allocator);

    bootstage::set_postboot();
    logger::disable();
    let real_mmap = unsafe {
        // SAFETY: Only thing we used was the UEFI console logger, and allocator, they are now disabled
        boot::exit_boot_services(MemoryType::LOADER_DATA)
    };
    let mut mmap = MemoryMap::<256>::new();
    let mut loader_mmap = MemoryMap::<64>::new();
    for entry in real_mmap.entries() {
        let region = MemoryRegion::new(
            PhysAddr::new_truncate(entry.phys_start as usize),
            MemorySize::new(entry.page_count as usize * 4096),
        );
        if entry.phys_start >= 1024 * 1024
            && (entry.ty == MemoryType::CONVENTIONAL
                || entry.ty == MemoryType::BOOT_SERVICES_CODE
                || entry.ty == MemoryType::BOOT_SERVICES_DATA)
        {
            mmap.add(region);
        }
        if entry.phys_start >= 1024 * 1024
            && (entry.ty == MemoryType::LOADER_CODE || entry.ty == MemoryType::LOADER_DATA)
        {
            loader_mmap.add(region);
        }
    }

    let allocator = unsafe {
        // SAFETY: We didn't include any memory under 1M, and LOADER_* memory in mmap
        PostBootAllocator::init(mmap)
    };

    // TODO: Setup virtual memory for kernel

    let _mmap = allocator.fini(loader_mmap);

    // TODO: Jump to kernel
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
