use crate::acpi;
use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use crate::allocator::PreBootAllocator;
use crate::bootstage;
use crate::features;
use crate::framebuffer;
use crate::kernel;
use crate::logger;
use crate::phys_mmap::PhysMemMap;
use crate::topology;
use crate::virt_mmap;
use boot_protocol::BootInfo;
use boot_protocol::MAX_MMAP_SIZE;
use boot_protocol::OFFSET_MAPPING;
use core::arch::asm;
use core::hint;
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
        // SAFETY: We call logger::disable() before exiting UEFI boot services.
        logger::init();
    }
    info!("Booting PentOS...");

    let features = features::bsp_featureset();
    let allocator = PreBootAllocator;
    acpi::init();
    let kernel = kernel::load_kernel(&allocator);

    topology::dump();

    loop {
        hint::spin_loop();
    }
    // Keep this last in PreBootStage
    let primary_framebuffer_info = framebuffer::init();

    logger::disable();
    bootstage::set_postboot();
    // TODO: AP wait_for_config
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
    unsafe {
        asm! {
            "out dx, al",
            in("dx") 0x402,
            in("al") 8u8
        };
    }
    let stack = kernel::alloc_stack(root_map, &mut allocator);
    root_map.load();
    let mmap = allocator.fini(loader_mmap);
    bootinfo.mmap = mmap.regions;
    bootinfo.mmap_len = mmap.len;

    kernel::bsp_cede_control(&kernel, stack);
}
