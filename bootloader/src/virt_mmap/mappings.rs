use {
    crate::{
        allocator::PostBootAllocator,
        phys_mmap::PhysMemMap,
        virt_mmap::{map, map_many},
    },
    boot_protocol::BootInfo,
    config::{pmem::IDENTITY_MAPPED_REGION, vmem::KBIN_REGION},
    core::{cmp::min, mem},
    elf::{Elf, SegmentType},
    log::debug,
    x64::{
        mem::{
            VirtualMemoryRegion,
            addr::{Address, PhysAddr},
            frame::{
                Frame,
                size::{Frame1GiB, Frame2MiB, Frame4KiB, FrameSize, FrameSizeOps},
            },
            page::{
                Page,
                size::{Page1GiB, Page2MiB, Page4KiB, PageSize},
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
        debug!("New entry ----------------");
        debug!("Region: {region:?}");

        let k4_start = *region.start();
        let k4_end = *region.end();
        let total_count = *region.size() / Frame4KiB::SIZE;
        debug!("K4TT: {total_count}");

        let m2_start = usize::min(k4_start.next_multiple_of(Frame2MiB::SIZE), k4_end);
        let g1_start = usize::min(k4_start.next_multiple_of(Frame1GiB::SIZE), k4_end);
        debug!("K4S: {k4_start:x},\nM2S: {m2_start:x},\nG1S: {g1_start:x}");

        let m2_end = usize::max(k4_end >> Frame2MiB::SHIFT << Frame2MiB::SHIFT, m2_start);
        let g1_end = usize::max(k4_end >> Frame1GiB::SHIFT << Frame1GiB::SHIFT, g1_start);
        debug!("K4E: {k4_end:x},\nM2E: {m2_end:x},\nG1E: {g1_end:x}");

        let leading_4k_count = (m2_start - k4_start) / Frame4KiB::SIZE;
        let leading_2m_count = (g1_start - m2_start) / Frame2MiB::SIZE;
        debug!("K4L: {leading_4k_count},\nM2L: {leading_2m_count}");

        let trailing_2m_count = (m2_end - usize::min(g1_end, m2_end)) / Frame2MiB::SIZE;
        let trailing_4k_count = (k4_end - m2_end) / Frame4KiB::SIZE;
        debug!("K4T: {trailing_4k_count},\nM2T: {trailing_2m_count}");

        let g1_count = (total_count
            - (leading_4k_count + trailing_4k_count)
            - (leading_2m_count + trailing_2m_count)
                * FrameSizeOps::<Frame2MiB, Frame4KiB>::FRAME_COUNT_DIFF)
            / FrameSizeOps::<Frame1GiB, Frame4KiB>::FRAME_COUNT_DIFF;
        debug!("G1C: {g1_count}");

        map_many::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            leading_4k_count,
            k4_start,
            offset,
        );
        map_many::<Page4KiB, ALLOCATOR_CAP>(map_root, allocator, trailing_4k_count, m2_end, offset);
        map_many::<Page2MiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            leading_2m_count,
            m2_start,
            offset,
        );
        map_many::<Page2MiB, ALLOCATOR_CAP>(map_root, allocator, trailing_2m_count, g1_end, offset);
        map_many::<Page1GiB, ALLOCATOR_CAP>(map_root, allocator, g1_count, g1_start, offset);
    }
}

pub fn apply_bootinfo_mapping<const ALLOCATOR_CAP: usize>(
    bootinfo: &BootInfo,
    target: Page<Page4KiB>,
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) {
    let bootinfo = Frame::containing(PhysAddr::new_panic(bootinfo as *const _ as usize));
    let pg_count = mem::size_of::<BootInfo>().div_ceil(Page4KiB::SIZE);
    for i in 0..pg_count {
        let frame = bootinfo + i;
        let page = target + i;
        map(
            root_map,
            allocator,
            frame,
            page,
            false,
            false,
            PatMemoryType::WriteBack,
        );
    }
}

pub fn apply_kbin_mapping<const ALLOCATOR_CAP: usize>(
    kernel: &Elf<'static>,
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) {
    debug!("Mapping kernel");
    for segment in &kernel.program_header {
        if segment.ty == SegmentType::Load {
            debug!("LOAD {vadr}", vadr = segment.vaddr);
            let pg_count = segment.mem_size.next_multiple_of(4096) / 4096;
            let mut copied = 0;
            for i in 0..pg_count {
                let segment_region = VirtualMemoryRegion::new(segment.vaddr, segment.mem_size);
                if !KBIN_REGION.contains_region(segment_region) {
                    panic!("Kernel binary has code/data outside required kernel region.");
                }

                let frame = Frame::containing(PhysAddr::new_panic(
                    allocator.alloc([0; 4096]).expect("Out of memory") as *const _ as usize,
                ));

                if copied < segment.file_size {
                    let src = kernel.data.as_ptr() as u64 + segment.offset + copied as u64;
                    let dst = frame.boundary();
                    let copy_amount = min(segment.file_size - copied, 4096);
                    unsafe {
                        // SAFETY: We are copying from a valid memory region to a valid memory region
                        core::ptr::copy_nonoverlapping(
                            src as *const u8,
                            dst.as_mut_ptr(),
                            copy_amount,
                        );
                    }
                    copied += copy_amount;
                }
                let page = Page::<Page4KiB>::containing(segment.vaddr + i * 4096);

                map(
                    root_map,
                    allocator,
                    frame,
                    page,
                    segment.flags.write,
                    segment.flags.exec,
                    PatMemoryType::WriteBack,
                );
            }
        }
    }
}
