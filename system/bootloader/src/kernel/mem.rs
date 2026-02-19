use {
    crate::{allocator::PostBootAllocator, topology, virt_mmap},
    boot_protocol::STACK_SIZE,
    utils::collections::smallvec::SmallVec,
    config::{topology::hart::MAX_HART_COUNT, vmem::KSTACK_REGION},
    x64::{
        mem::{
            addr::{Address, PhysAddr, VirtAddr},
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

/// # Safety
/// Should be called only once, and in the BSP
pub unsafe fn alloc_and_map_stacks<const ALLOCATOR_CAP: usize>(
    map_root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> SmallVec<VirtAddr, MAX_HART_COUNT> {
    // We will allocate as many stacks as harts on the system
    // And leave gaps between them to cause a page fault if a stack ever runs out
    // The gaps will be as large as a single stack
    // We allocate stacks starting from the highest address, leaving a gap at the
    // end as well
    let mut stacks = SmallVec::new();
    let hart_count = topology::topology().harts.len();
    let mut current_stack = KSTACK_REGION.end();
    let pg_count = STACK_SIZE / Page4KiB::SIZE;

    assert!(hart_count <= MAX_HART_COUNT);
    assert!(
        hart_count * 2 * STACK_SIZE < KSTACK_REGION.size().as_usize(),
        "Cannot fit kernel stacks for the number of harts"
    );

    for _ in 0..hart_count {
        let stack_ptr = current_stack - STACK_SIZE;
        current_stack -= 2 * STACK_SIZE;

        let phys_stack = unsafe {
            // SAFETY: All u8 are valid
            allocator
                .alloc_slice::<u8>(STACK_SIZE)
                .expect("Couldn't allocate a kernel stack")
                .assume_init_mut()
        };
        phys_stack.fill(0); // Not really needed, but what we losing (time, time is money)

        let frame = Frame::containing(PhysAddr::new_panic(phys_stack.as_ptr() as usize));
        let page = Page::containing(stack_ptr - STACK_SIZE);

        virt_mmap::map_many::<Page4KiB, ALLOCATOR_CAP>(
            map_root,
            allocator,
            frame,
            page,
            pg_count,
            true,  // WRITE
            false, // EXEC
            MemoryType::WriteBack,
        );

        unsafe {
            // SAFETY: We checked that hart_count is less than the maximum
            stacks.push(stack_ptr).unwrap_unchecked()
        };
    }

    stacks
}
