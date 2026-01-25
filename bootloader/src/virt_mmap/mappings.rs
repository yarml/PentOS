use {
    crate::{
        allocator::PostBootAllocator,
        phys_mmap::PhysMemMap,
        virt_mmap::{map_many, map_optimal},
    },
    boot_protocol::BootInfo,
    config::{
        pmem::IDENTITY_MAPPED_REGION,
        vmem::{BOOTINFO_REGION, KBIN_REGION},
    },
    core::{cmp::min, mem, slice},
    elf::{Elf, SegmentType},
    log::debug,
    x64::{
        mem::{
            VirtualMemoryRegion,
            addr::{Address, PhysAddr, VirtAddr},
            frame::Frame,
            page::{
                Page,
                size::{Page4KiB, PageSize},
            },
            paging::PagingRootEntry,
        },
        msr::pat::MemoryType as PatMemoryType,
    },
};

pub fn apply_id_and_off_mapping<const ALLOCATOR_CAP: usize, const MMAP_CAP: usize>(
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

pub fn apply_bootinfo_mapping<const ALLOCATOR_CAP: usize>(
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

pub fn apply_kbin_mapping<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    kernel: &Elf<'static>,
) {
    debug!("Mapping kernel");
    for segment in &kernel.program_header {
        if segment.ty == SegmentType::Load {
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
            let pg_count = segment.mem_size.next_multiple_of(4096) / 4096;
            let segment_mem = unsafe {
                // SAFETY: Any u8 is valid
                allocator
                    .alloc_slice(segment.mem_size.as_usize())
                    .expect("Couldn't allocate memory to load kernel")
                    .assume_init_mut()
            };
            let segment_image = unsafe {
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
}
