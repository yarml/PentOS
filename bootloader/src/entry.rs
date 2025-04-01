use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use crate::allocator::PreBootAllocator;
use crate::bootstage;
use crate::features;
use crate::framebuffer;
use crate::kernel;
use crate::logger;
use crate::phys_mmap::PhysMemMap;
use crate::virt_mmap;
use boot_protocol::BootInfo;
use boot_protocol::MAX_MMAP_SIZE;
use boot_protocol::OFFSET_MAPPING;
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
use x64::mem::addr::VirtAddr;
use x64::mem::page::Page;
use x64::msr::efer::Efer;
use x64::msr::pat::standard_pat;

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

    let features = features::featureset();
    let allocator = PreBootAllocator;
    let kernel = kernel::load_kernel(&allocator);

    // Keep this last in PreBootStage
    let primary_framebuffer_info = framebuffer::init();
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

    Efer::new().syscall(false).exec_disable(true).write();
    standard_pat().write();
    let root_map =
        virt_mmap::identity_and_offset_mapping(&mut allocator, &real_mmap, OFFSET_MAPPING);
    kernel::map_kernel(&kernel, root_map, &mut allocator);
    let framebuffer =
        framebuffer::postboot_init(primary_framebuffer_info, root_map, &mut allocator);

    let bootinfo = BootInfo {
        mmap: [MemoryRegion::null(); MAX_MMAP_SIZE],
        mmap_len: 0,
        features,
        framebuffer,
    };
    let bootinfo = allocator
        .alloc(bootinfo)
        .expect("Failed to allocate bootinfo");
    virt_mmap::map_bootinfo(
        bootinfo,
        Page::containing(VirtAddr::new_truncate(OFFSET_MAPPING)),
        root_map,
        &mut allocator,
    );
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
