use {
    crate::{allocator::PostBootAllocator, phys_mmap::PhysMemMap},
    boot_protocol::BootInfo,
    config::pmem::IDENTITY_MAPPED_REGION,
    core::mem,
    log::debug,
    x64::{
        mem::{
            MemorySize,
            addr::{Address, PhysAddr, VirtAddr},
            frame::{
                Frame,
                size::{Frame1GiB, Frame2MiB, Frame4KiB, FrameSize, FrameSizeOps},
            },
            page::{
                Page,
                size::{Page1GiB, Page2MiB, Page4KiB, Page512GiB, PageSize, PageSizeMap},
            },
            paging::{PagingMapEntry, PagingRawEntry, PagingReferenceEntry, PagingRootEntry},
        },
        msr::pat::{MemoryType as PatMemoryType, pat_index},
    },
};

pub struct VirtMemMap {}

pub fn map<PS: PageSizeMap, const ALLOCATOR_CAP: usize>(
    root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    frame: Frame<PS::PhysicalPageSize>,
    page: Page<PS>,
    write: bool,
    exec: bool,
    mtype: PatMemoryType,
) {
    let pml4t = unsafe { root.target_mut() };
    let pml4e = pml4t[page.order_index::<Page512GiB>()].as_raw();

    let pdpt = target_or_alloc(pml4e, allocator);
    let pdpe = &mut pdpt[page.order_index::<Page1GiB>()];

    if PS::SIZE == Page1GiB::SIZE {
        *pdpe = make_map_entry(frame.resize(), write, exec, mtype);
        return;
    }

    let pdt = target_or_alloc(pdpe, allocator);
    let pde = &mut pdt[page.order_index::<Page2MiB>()];

    if PS::SIZE == Page2MiB::SIZE {
        *pde = make_map_entry(frame.resize(), write, exec, mtype);
        return;
    }

    let pt = target_or_alloc(pde, allocator);
    let pe = &mut pt[page.order_index::<Page4KiB>()];

    if PS::SIZE == Page4KiB::SIZE {
        *pe = make_map_entry(frame.resize(), write, exec, mtype);
        return;
    }

    // Pages of non standard size
    unimplemented!()
}

pub fn new_root<const ALLOCATOR_CAP: usize>(
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> PagingRootEntry {
    let target = allocator
        .alloc([PagingRawEntry::<Page512GiB>::new(0); 512])
        .expect("Out of memory");
    PagingRootEntry::new(Frame::containing(PhysAddr::new_panic(
        target as *const _ as usize,
    )))
}

fn target_or_alloc<'a, PS, const ALLOCATOR_CAP: usize>(
    entry: &mut PagingRawEntry<PS>,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> &'a mut [PagingRawEntry<PS::ReferenceTarget>]
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    if let Some(entry_reference) = entry.as_reference() {
        unsafe {
            // SAFETY: trust in the process
            entry_reference.target_mut()
        }
    } else if entry.as_absent().is_some() {
        let target = allocator
            .alloc([PagingRawEntry::new(0); 512])
            .expect("Out of memory");
        let reference = PagingReferenceEntry::<PS>::new(Frame::containing(PhysAddr::new_panic(
            target as *const _ as usize,
        )))
        .write()
        .exec()
        .to_raw();
        *entry = reference;
        target
    } else {
        unimplemented!()
    }
}

fn make_map_entry<PS1: PageSizeMap>(
    frame: Frame<PS1::PhysicalPageSize>,
    write: bool,
    exec: bool,
    mtype: PatMemoryType,
) -> PagingRawEntry<PS1> {
    let mut new_entry = PagingMapEntry::new(frame).with_pat_index(pat_index(mtype));

    if write {
        new_entry = new_entry.write();
    }

    if exec {
        new_entry = new_entry.exec();
    }
    new_entry.to_raw()
}

pub fn identity_and_offset_mapping<const ALLOCATOR_CAP: usize, const MMAP_CAP: usize>(
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    mmap: &PhysMemMap<MMAP_CAP>,
    offset: usize,
) -> PagingRootEntry {
    let root_map = new_root(allocator);

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

        let m2_end = k4_end >> Frame2MiB::SHIFT << Frame2MiB::SHIFT;
        let g1_end = k4_end >> Frame1GiB::SHIFT << Frame1GiB::SHIFT;
        debug!("K4E: {k4_end:x},\nM2E: {m2_end:x},\nG1E: {g1_end:x}");

        let leading_4k_count = (m2_start - k4_start) / Frame4KiB::SIZE;
        let leading_2m_count = (g1_start - m2_start) / Frame2MiB::SIZE;
        debug!("K4L: {leading_4k_count},\nM2L: {leading_2m_count}");

        let trailing_2m_count = (k4_end - m2_end) / Frame2MiB::SIZE;
        let trailing_4k_count = (k4_end - g1_end) / Frame1GiB::SIZE;
        debug!("K4T: {trailing_4k_count},\nM2T: {trailing_2m_count}");

        let g1_count = (total_count
            - (leading_4k_count + trailing_4k_count)
            - (leading_2m_count + trailing_2m_count)
                * FrameSizeOps::<Frame2MiB, Frame4KiB>::FRAME_COUNT_DIFF)
            / FrameSizeOps::<Frame1GiB, Frame4KiB>::FRAME_COUNT_DIFF;
        debug!("G1C: {g1_count}");

        map_many::<Page4KiB, ALLOCATOR_CAP>(root_map, allocator, leading_4k_count, k4_start, offset);
        map_many::<Page4KiB, ALLOCATOR_CAP>(root_map, allocator, trailing_4k_count, m2_end, offset);
        map_many::<Page2MiB, ALLOCATOR_CAP>(root_map, allocator, leading_2m_count, m2_start, offset);
        map_many::<Page2MiB, ALLOCATOR_CAP>(root_map, allocator, trailing_2m_count, g1_end, offset);
        map_many::<Page1GiB, ALLOCATOR_CAP>(root_map, allocator, g1_count, g1_start, offset);
    }

    root_map
}

fn map_many<PS: PageSize, const ALLOCATOR_CAP: usize>(
    root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    count: usize,
    start: usize,
    offset: usize,
) {
    for pi in 0..count {
        let start_adr = start + pi * PS::SIZE;
        let frame = Frame::containing(PhysAddr::new_panic(start_adr));

        let identity_vaddr = VirtAddr::new_panic(start_adr);
        let offset_vadr = identity_vaddr + offset;
        let identity_page = Page::containing(identity_vaddr);
        let offset_page = Page::containing(offset_vadr);

        // Mapping identity as executable, since we still have the bootloader here
        map::<PS, ALLOCATOR_CAP>(
            root,
            allocator,
            frame,
            identity_page,
            true, // WRITE
            true, // EXEC
            PatMemoryType::WriteBack,
        );

        // Mapping the offset as non executable, since the kernel will be mapped on its own elsewhere
        map::<PS, ALLOCATOR_CAP>(
            root,
            allocator,
            frame,
            offset_page,
            true,  // WRITE
            false, // NO EXEC
            PatMemoryType::WriteBack,
        );
    }
}

pub fn map_bootinfo<const ALLOCATOR_CAP: usize>(
    bootinfo: &BootInfo,
    target: Page<Page4KiB>,
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) {
    let bootinfo = Frame::containing(PhysAddr::new_panic(bootinfo as *const _ as usize));
    let pg_count = mem::size_of::<BootInfo>().next_multiple_of(4096) / 4096;
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
