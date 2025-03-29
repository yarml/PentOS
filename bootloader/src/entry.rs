use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use crate::allocator::PreBootAllocator;
use crate::bootstage;
use crate::features;
use crate::kernel;
use crate::logger;
use crate::phys_mmap::PhysMemMap;
use crate::virt_mmap;
use core::arch::asm;
use elf::SegmentType;
use log::debug;
use log::info;
use uefi::Status;
use uefi::boot;
use uefi::boot::MemoryAttribute;
use uefi::boot::MemoryType;
use uefi::entry;
use uefi::mem::memory_map::MemoryMap as UefiMemoryMap;
use uefi::system;
use x64::mem::MemoryRegion;
use x64::mem::MemorySize;
use x64::mem::addr::PhysAddr;
use x64::mem::frame::Frame;
use x64::mem::page::Page;
use x64::mem::paging::PagingRootEntry;

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

    let featureset = features::featureset();

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
    let mut mmap = PhysMemMap::<ALLOCATOR_CAP>::new();
    let mut loader_mmap = PhysMemMap::<64>::new();
    for entry in real_mmap.entries() {
        let region = MemoryRegion::new(
            PhysAddr::new_truncate(entry.phys_start as usize),
            MemorySize::new(entry.page_count as usize * 4096),
        );
        if entry.phys_start >= 1024 * 1024 && (entry.ty == MemoryType::CONVENTIONAL) {
            mmap.add(region);
        }
        if entry.phys_start >= 1024 * 1024
            && (entry.ty == MemoryType::LOADER_CODE
                || entry.ty == MemoryType::LOADER_DATA
                || entry.ty == MemoryType::BOOT_SERVICES_CODE
                || entry.ty == MemoryType::BOOT_SERVICES_DATA)
        {
            loader_mmap.add(region);
        }
    }

    let mut allocator = unsafe {
        // SAFETY: We didn't include any memory under 1M, nor LOADER_* memory in mmap
        PostBootAllocator::init(mmap)
    };

    let root_map = virt_mmap::new_root(&mut allocator);
    // TODO: Setup virtual memory for kernel

    for entry in real_mmap.entries() {
        let region = MemoryRegion::new(
            PhysAddr::new_truncate(entry.phys_start as usize),
            MemorySize::new(entry.page_count as usize * 4096),
        );
        if entry.phys_start >= 1024 * 1024
            && (entry.ty == MemoryType::CONVENTIONAL
                || entry.ty == MemoryType::BOOT_SERVICES_CODE
                || entry.ty == MemoryType::BOOT_SERVICES_DATA
                || entry.ty == MemoryType::LOADER_CODE
                || entry.ty == MemoryType::LOADER_DATA)
        {
            let pg_count = *region.size() / 4096;
            for i in 0..pg_count {
                let frame = Frame::containing(region.start() + i * 4096);
                let page = Page::containing(region.start().to_virt() + i * 4096);
                virt_mmap::map(root_map, &mut allocator, frame, page);
            }
        }
    }

    debug!("Done identity mapping");

    for segment in &kernel.program_header {
        if segment.ty == SegmentType::Load {
            virt_mmap::map(
                root_map,
                &mut allocator,
                Frame::containing(PhysAddr::new_truncate(0)),
                Page::containing(segment.vaddr),
            );
        }
    }

    debug!("Loading new mapping");

    root_map.load();
    let _mmap = allocator.fini(loader_mmap);

    debug!("Check monitor");

    // TODO: Jump to kernel
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}
