use crate::allocator::PostBootAllocator;
use crate::allocator::PreBootAllocator;
use crate::bootstage;
use crate::kernel;
use crate::logger;
use crate::phys_mmap::PhysMemMap;
use core::arch::asm;
use log::debug;
use log::info;
use uefi::Status;
use uefi::boot;
use uefi::boot::MemoryType;
use uefi::entry;
use uefi::mem::memory_map::MemoryMap as UefiMemoryMap;
use uefi::system;
use x64::mem::MemoryRegion;
use x64::mem::MemorySize;
use x64::mem::addr::PhysAddr;

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
    debug!("Kernel entry point: {:#x}", kernel.entry.as_u64());
    for segment in &kernel.program_header {
        debug!("Segment {:x?}:", segment);
    }

    logger::disable();
    bootstage::set_postboot();
    let real_mmap = unsafe {
        // SAFETY: Only thing we used was the UEFI console logger, and allocator, they are now disabled
        boot::exit_boot_services(MemoryType::LOADER_DATA)
    };
    let mut mmap = PhysMemMap::<256>::new();
    let mut loader_mmap = PhysMemMap::<64>::new();
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
        // SAFETY: We didn't include any memory under 1M, nor LOADER_* memory in mmap
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
