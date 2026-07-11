use {
    crate::{
        allocator::PostBootAllocator,
        phys_mmap::PhysMemMap,
        topology,
        virt_mmap::{map, map_many, map_optimal},
    },
    boot_protocol::{BootInfo, topology::PCIConfigSpace},
    core::{cmp::min, mem, slice},
    elf::{Elf, SegmentType},
    log::debug,
    pci::adr::{BusAddress, DEV_PER_BUS, FUNC_PER_DEV},
    system::{
        pmem::IDENTITY_MAPPED_REGION,
        vmem::{BOOTINFO_REGION, IOAPIC_REGION, KBIN_REGION},
    },
    x64::{
        mem::{
            VirtualMemoryRegion,
            addr::{Address, PhysAddr, VirtAddr},
            frame::{
                Frame,
                size::{Frame4KiB, FrameSize},
            },
            page::{
                Page,
                size::{Page4KiB, PageSize},
            },
            paging::PagingRootEntry,
        },
        msr::pat::MemoryType as PatMemoryType,
    },
};

/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn apply_id_and_off_mapping<const ALLOCATOR_CAP: usize, const MMAP_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    mmap: &PhysMemMap<MMAP_CAP>,
    offset: usize,
) {
    for region in mmap {
        let region = region.intersect(IDENTITY_MAPPED_REGION);

        if region.is_null() {
            continue;
        }

        let identity_region = VirtualMemoryRegion::new(
            VirtAddr::new_panic(region.start().as_usize()),
            region.size(),
        );
        let offset_region = VirtualMemoryRegion::new(
            VirtAddr::new_panic(region.start().as_usize() + offset),
            region.size(),
        );

        map_optimal(
            map_root,
            allocator,
            identity_region,
            region.start(),
            true,
            true,
            PatMemoryType::WriteBack,
        );
        map_optimal(
            map_root,
            allocator,
            offset_region,
            region.start(),
            true,
            false,
            PatMemoryType::WriteBack,
        );
    }
}

/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn apply_bootinfo_mapping<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    bootinfo: &BootInfo,
) {
    let bootinfo = Frame::containing(PhysAddr::new_panic(bootinfo as *const _ as usize));
    let target = Page::containing(VirtAddr::new_panic(BOOTINFO_REGION.start().as_usize()));
    let pg_count = mem::size_of::<BootInfo>().div_ceil(Page4KiB::SIZE);
    map_many::<Page4KiB, ALLOCATOR_CAP>(
        map_root,
        allocator,
        bootinfo,
        target,
        pg_count,
        false,
        false,
        PatMemoryType::WriteBack,
    );
}

/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn apply_kbin_mapping<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    kernel: &Elf<'static>,
) {
    debug!("Mapping kernel");
    for segment in kernel
        .program_header
        .into_iter()
        .filter(|s| s.ty == SegmentType::Load)
    {
        debug!(
            "LOAD {vadr} {size}",
            vadr = segment.vaddr,
            size = segment.mem_size
        );
        let segment_region = VirtualMemoryRegion::new(segment.vaddr, segment.mem_size);
        if !KBIN_REGION.contains_region(segment_region) {
            panic!("Kernel binary has code/data outside required kernel region.");
        }
        if segment.file_size > segment.mem_size.as_usize() {
            panic!("Kernel has a segment with larger file size than memory size");
        }
        let pg_count = segment.mem_size.next_multiple_of(Frame4KiB::SIZE) / Frame4KiB::SIZE;
        let segment_mem = unsafe {
            // SAFETY: Any u8 is valid
            allocator
                .alloc_slice(segment.mem_size.as_usize())
                .expect("Couldn't allocate memory to load kernel")
                .assume_init_mut()
        };
        let segment_image = unsafe {
            // SAFETY: trusting kernel binary for now
            // TODO: sanitize in file offsets
            slice::from_raw_parts(
                kernel.data.as_ptr().add(segment.offset as usize),
                segment.file_size,
            )
        };
        let copy_amount = min(segment.file_size, segment.mem_size.as_usize());
        segment_mem[..copy_amount].copy_from_slice(&segment_image[..copy_amount]);
        segment_mem[copy_amount..].fill(0);

        map_many::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            Frame::containing(PhysAddr::new_panic(segment_mem.as_ptr() as usize)),
            Page::containing(segment.vaddr),
            pg_count,
            segment.flags.write,
            segment.flags.exec,
            PatMemoryType::WriteBack,
        );
    }
}

/// This is needed for waking up harts
/// We do not offset map legacy memory (< 1MiB)
/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn apply_legacy_mem_mapping<const ALLOCATOR_CAP: usize, const LEGACY_MMAP_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    legacy_mmap: &PhysMemMap<LEGACY_MMAP_CAP>,
) {
    for region in legacy_mmap {
        let phys_start = Frame::containing(region.start());
        let virt_start = Page::containing(VirtAddr::new_panic(region.start().as_usize()));
        let pg_count = region.size().as_usize() / Page4KiB::SIZE;
        map_many::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            phys_start,
            virt_start,
            pg_count,
            true, // WRITE
            true, // EXEC
            PatMemoryType::WriteBack,
        );
    }
}

/// Maps I/O APICS at id * 4K within IOAPIC_REGION
/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn apply_ioapic_mappings<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) {
    let ioapics = &topology::topology().int_controllers;
    for ioapic in ioapics {
        let base = system::ioapic::standard_addressof(ioapic.id);
        if !IOAPIC_REGION.contains(base) {
            panic!("I/O APIC ID too large making its address outside the I/O APIC region");
        }
        let frame = Frame::containing(ioapic.register_base);
        let page = Page::containing(base);
        map::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            frame,
            page,
            true,
            false,
            PatMemoryType::Uncacheable,
        );
    }
}

/// Maps PCIe MMIO configuration space into PCIE_REGION
/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn apply_pcie_mappings<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    cs_phys_map: &[PCIConfigSpace],
) {
    for cs in cs_phys_map {
        let bus_count = cs.bus_end - cs.bus_start + 1;

        let bus_start_addr = BusAddress::new(cs.segment_group, cs.bus_start);
        let bus_end_addr = BusAddress::new(cs.segment_group, cs.bus_end);

        let ecam_region = VirtualMemoryRegion::new_boundaries(
            bus_start_addr.start_virtaddr(),
            bus_end_addr.end_virtaddr(),
        );

        map_many::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            Frame::containing(cs.phys_base),
            Page::containing(ecam_region.start()),
            bus_count * DEV_PER_BUS * FUNC_PER_DEV,
            true,
            false,
            PatMemoryType::Uncacheable,
        );
    }
}
