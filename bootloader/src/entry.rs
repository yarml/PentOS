use {
    crate::{
        acpi,
        allocator::{ALLOCATOR_CAP, PostBootAllocator, PreBootAllocator},
        bootstage, features, framebuffer, kernel, loader, logger,
        phys_mmap::PhysMemMap,
        pic, topology, virt_mmap,
    },
    boot_protocol::{BootInfo, MAX_MMAP_SIZE, OFFSET_MAPPING},
    log::{debug, info},
    uefi::{
        Status,
        boot::{self, MemoryType},
        entry,
        mem::memory_map::MemoryMap as UefiMemoryMap,
        system,
    },
    x64::{
        mem::{
            MemorySize, PhysicalMemoryRegion,
            addr::{Address, PhysAddr, VirtAddr},
            page::Page,
        },
        msr::{efer::Efer, pat::standard_pat},
    },
};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    system::with_stdout(|stdout| {
        // If it fails, we don't really care.
        stdout.clear().ok();
    });
    logger::init();
    loader::init();
    info!("Booting PentOS...");

    debug!("Bootloader base: {}", loader::base());
    debug!("efi_main: {}", PhysAddr::new(main as *const () as usize).unwrap());

    let features = features::bsp_featureset();
    let allocator = PreBootAllocator;
    acpi::init();
    let kernel = kernel::load_kernel(&allocator);

    topology::dump();

    // Keep this last in PreBootStage
    let primary_framebuffer_info = framebuffer::init();

    bootstage::set_postboot();
    let uefi_mmap = unsafe {
        // SAFETY: Only thing we used was the UEFI console logger, and allocator, they are now disabled
        boot::exit_boot_services(Some(MemoryType::LOADER_DATA))
    };

    pic::disable();

    debug!("PIC disabled");

    // The difference between real_mmap and mmap is that mmap is moved to the allocator
    // real_mmap is exclusively used to identity & offset map memory
    let mut real_mmap = PhysMemMap::<MAX_MMAP_SIZE>::new();
    let mut mmap = PhysMemMap::<ALLOCATOR_CAP>::new();

    let mut loader_mmap = PhysMemMap::<64>::new();
    for entry in uefi_mmap.entries() {
        let region = PhysicalMemoryRegion::new(
            PhysAddr::new_panic(entry.phys_start as usize),
            MemorySize::new(entry.page_count as usize * 4096),
        );
        debug!("{region} - {ty:?}", ty = entry.ty);
        if entry.phys_start >= 1024 * 1024 && (entry.ty == MemoryType::CONVENTIONAL) {
            mmap.add(region);
            real_mmap.add(region);
        }
        if entry.phys_start >= 1024 * 1024
            && (entry.ty == MemoryType::LOADER_CODE
                || entry.ty == MemoryType::LOADER_DATA
                || entry.ty == MemoryType::BOOT_SERVICES_CODE
                || entry.ty == MemoryType::BOOT_SERVICES_DATA)
        {
            loader_mmap.add(region);
            real_mmap.add(region);
        }
    }

    let mut allocator = unsafe {
        // SAFETY: We didn't include any memory under 1M, nor LOADER_* memory in mmap
        PostBootAllocator::init(mmap)
    };

    standard_pat().write();

    let map_root = virt_mmap::paging_root_new(&mut allocator);

    virt_mmap::apply_id_and_off_mapping(map_root, &mut allocator, &real_mmap, OFFSET_MAPPING);
    virt_mmap::apply_kbin_mapping(&kernel, map_root, &mut allocator);

    let framebuffer =
        framebuffer::postboot_init(primary_framebuffer_info, map_root, &mut allocator);

    let bootinfo = BootInfo {
        mmap: [PhysicalMemoryRegion::null(); MAX_MMAP_SIZE],
        mmap_len: 0,
        features,
        framebuffer,
    };
    let bootinfo = allocator
        .alloc(bootinfo)
        .expect("Failed to allocate bootinfo");
    virt_mmap::apply_bootinfo_mapping(
        bootinfo,
        Page::containing(VirtAddr::new_panic(OFFSET_MAPPING)),
        map_root,
        &mut allocator,
    );
    let stack = kernel::alloc_stack(map_root, &mut allocator);
    map_root.load();
    debug!("Initialized kernel memory map");

    Efer::new().syscall(false).exec_disable(true).write();

    let mmap = allocator.fini(loader_mmap);
    bootinfo.mmap = mmap.regions;
    bootinfo.mmap_len = mmap.len;
    debug!("Ceding control to kernel");
    kernel::bsp_cede_control(&kernel, stack, bootinfo);
}
