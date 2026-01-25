use {
    crate::{
        allocator::PostBootAllocator,
        virt_mmap::{page_map_new, page_target_or_new},
    },
    x64::{
        mem::{
            VirtualMemoryRegion,
            addr::{Address, PhysAddr, VirtAddr},
            frame::{
                Frame,
                size::{Frame1GiB, Frame2MiB, Frame4KiB, FrameSize, FrameSizeOps},
            },
            page::{
                Page,
                size::{Page1GiB, Page2MiB, Page4KiB, Page512GiB, PageSize, PageSizeMap},
            },
            paging::PagingRootEntry,
        },
        msr::pat::MemoryType as PatMemoryType,
    },
};

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

    let pdpt = page_target_or_new(pml4e, allocator);
    let pdpe = &mut pdpt[page.order_index::<Page1GiB>()];

    if PS::SIZE == Page1GiB::SIZE {
        *pdpe = page_map_new(frame.resize(), write, exec, mtype);
        return;
    }

    let pdt = page_target_or_new(pdpe, allocator);
    let pde = &mut pdt[page.order_index::<Page2MiB>()];

    if PS::SIZE == Page2MiB::SIZE {
        *pde = page_map_new(frame.resize(), write, exec, mtype);
        return;
    }

    let pt = page_target_or_new(pde, allocator);
    let pe = &mut pt[page.order_index::<Page4KiB>()];

    if PS::SIZE == Page4KiB::SIZE {
        *pe = page_map_new(frame.resize(), write, exec, mtype);
        return;
    }

    // Pages of non standard size
    unimplemented!()
}

#[allow(clippy::too_many_arguments)]
pub fn map_many<PS: PageSize, const ALLOCATOR_CAP: usize>(
    root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    phys_start: Frame<PS::PhysicalPageSize>,
    virt_start: Page<PS>,
    count: usize,
    write: bool,
    exec: bool,
    mtype: PatMemoryType,
) {
    for pi in 0..count {
        let phys_start = phys_start + pi;
        let virt_start = virt_start + pi;

        map::<PS, ALLOCATOR_CAP>(root, allocator, phys_start, virt_start, write, exec, mtype);
    }
}

pub fn map_optimal<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    virt_region: VirtualMemoryRegion,
    phys_start: PhysAddr,
    write: bool,
    exec: bool,
    mtype: PatMemoryType,
) {
    if virt_region.is_null() {
        return;
    }

    if phys_start.trailing_zeros() < virt_region.start().trailing_zeros() {
        todo!("Handle alignment when phys_start is less aligned than virt_region");
    }

    let k4_start = *virt_region.start();
    let k4_end = *virt_region.end();
    let total_count = *virt_region.size() / Frame4KiB::SIZE;

    let m2_start = usize::min(k4_start.next_multiple_of(Frame2MiB::SIZE), k4_end);
    let g1_start = usize::min(k4_start.next_multiple_of(Frame1GiB::SIZE), k4_end);

    let m2_end = usize::max(k4_end >> Frame2MiB::SHIFT << Frame2MiB::SHIFT, m2_start);
    let g1_end = usize::max(k4_end >> Frame1GiB::SHIFT << Frame1GiB::SHIFT, g1_start);

    let leading_4k_count = (m2_start - k4_start) / Frame4KiB::SIZE;
    let leading_2m_count = (g1_start - m2_start) / Frame2MiB::SIZE;

    let trailing_2m_count = (m2_end - usize::min(g1_end, m2_end)) / Frame2MiB::SIZE;
    let trailing_4k_count = (k4_end - m2_end) / Frame4KiB::SIZE;

    let g1_count = (total_count
        - (leading_4k_count + trailing_4k_count)
        - (leading_2m_count + trailing_2m_count)
            * FrameSizeOps::<Frame2MiB, Frame4KiB>::FRAME_COUNT_DIFF)
        / FrameSizeOps::<Frame1GiB, Frame4KiB>::FRAME_COUNT_DIFF;

    let k4_virt_start = Page::containing(VirtAddr::new_panic(k4_start));
    let m2_virt_start = Page::containing(VirtAddr::new_panic(m2_start));
    let g1_virt_start = Page::containing(VirtAddr::new_panic(g1_start));
    let m2_virt_end = Page::containing(VirtAddr::new_panic(m2_end));
    let g1_virt_end = Page::containing(VirtAddr::new_panic(g1_end));

    let k4_phys_start = Frame::containing(PhysAddr::new_panic(
        k4_start - virt_region.start().as_usize() + phys_start.as_usize(),
    ));
    let m2_phys_start = Frame::containing(PhysAddr::new_panic(
        m2_start - virt_region.start().as_usize() + phys_start.as_usize(),
    ));
    let g1_phys_start = Frame::containing(PhysAddr::new_panic(
        g1_start - virt_region.start().as_usize() + phys_start.as_usize(),
    ));
    let m2_phys_end = Frame::containing(PhysAddr::new_panic(
        m2_end - virt_region.start().as_usize() + phys_start.as_usize(),
    ));
    let g1_phys_end = Frame::containing(PhysAddr::new_panic(
        g1_end - virt_region.start().as_usize() + phys_start.as_usize(),
    ));

    map_many::<Page4KiB, ALLOCATOR_CAP>(
        map_root,
        allocator,
        k4_phys_start,
        k4_virt_start,
        leading_4k_count,
        write,
        exec,
        mtype,
    );
    map_many::<Page4KiB, ALLOCATOR_CAP>(
        map_root,
        allocator,
        m2_phys_end,
        m2_virt_end,
        trailing_4k_count,
        write,
        exec,
        mtype,
    );

    map_many::<Page2MiB, ALLOCATOR_CAP>(
        map_root,
        allocator,
        m2_phys_start,
        m2_virt_start,
        leading_2m_count,
        write,
        exec,
        mtype,
    );
    map_many::<Page2MiB, ALLOCATOR_CAP>(
        map_root,
        allocator,
        g1_phys_end,
        g1_virt_end,
        trailing_2m_count,
        write,
        exec,
        mtype,
    );

    map_many::<Page1GiB, ALLOCATOR_CAP>(
        map_root,
        allocator,
        g1_phys_start,
        g1_virt_start,
        g1_count,
        write,
        exec,
        mtype,
    );
}
