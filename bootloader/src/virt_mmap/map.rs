use {
    crate::{
        allocator::PostBootAllocator,
        virt_mmap::{page_map_new, page_target_or_new},
    },
    x64::{
        mem::{
            addr::{Address, PhysAddr, VirtAddr},
            frame::Frame,
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

pub fn map_many<PS: PageSize, const ALLOCATOR_CAP: usize>(
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
