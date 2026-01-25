use config::topology::hart::MAX_HART_COUNT;

use {
    boot_protocol::STACK_SIZE,
    x64::{
        mem::{
            addr::VirtAddr,
            frame::Frame,
            page::{
                Page,
                size::{Page4KiB, PageSize},
            },
            paging::PagingRootEntry,
        },
        msr::pat::MemoryType,
    },
};

use crate::{allocator::PostBootAllocator, infoarea::allocate_info_space, virt_mmap};

pub fn alloc_stack<const ALLOCATOR_CAP: usize>(
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> VirtAddr {
    let stack = Page::<Page4KiB>::containing(allocate_info_space(STACK_SIZE));
    let pg_count = STACK_SIZE.div_ceil(Page4KiB::SIZE);
    for i in 0..pg_count {
        let frame = Frame::containing(allocator.alloc_raw(0x1000, 0x1000).expect("Out of memory"));
        let page = stack + i;
        virt_mmap::map(
            root_map,
            allocator,
            frame,
            page,
            true,
            false,
            MemoryType::WriteBack,
        );
    }
    stack.boundary() + STACK_SIZE
}

pub fn alloc_and_map_stacks<const ALLOCATOR_CAP: usize>(
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>
) -> [VirtAddr; MAX_HART_COUNT] {
    todo!()
}
