use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use log::debug;
use x64::mem::MemorySize;
use x64::mem::addr::PhysAddr;
use x64::mem::frame::Frame;
use x64::mem::frame::size::Frame4KiB;
use x64::mem::page::Page;
use x64::mem::page::size::Page1GiB;
use x64::mem::page::size::Page2MiB;
use x64::mem::page::size::Page4KiB;
use x64::mem::page::size::Page512GiB;
use x64::mem::page::size::PageSize;
use x64::mem::paging::PagingMapEntry;
use x64::mem::paging::PagingRawEntry;
use x64::mem::paging::PagingReferenceEntry;
use x64::mem::paging::PagingRootEntry;

pub struct VirtMemMap {}

pub fn map(
    root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    frame: Frame<Frame4KiB>,
    page: Page<Page4KiB>,
) {
    let pml4_table = unsafe { root.target_mut() };

    let p4i = page.order_index::<Page512GiB>();
    let p3i = page.order_index::<Page1GiB>();
    let p2i = page.order_index::<Page2MiB>();
    let p1i = page.order_index::<Page4KiB>();

    let pdp_table = target_or_alloc(
        pml4_table[page.order_index::<Page512GiB>()].as_raw(),
        allocator,
    );
    let pd_table = target_or_alloc(&mut pdp_table[page.order_index::<Page1GiB>()], allocator);
    let pt_table = target_or_alloc(&mut pd_table[page.order_index::<Page2MiB>()], allocator);
    let pt_entry = &mut pt_table[page.order_index::<Page4KiB>()];

    *pt_entry = PagingMapEntry::<Page4KiB>::new(frame)
        .write()
        .exec()
        .to_raw();
}

pub fn new_root(allocator: &mut PostBootAllocator<ALLOCATOR_CAP>) -> PagingRootEntry {
    let target = allocator
        .alloc([PagingRawEntry::<Page512GiB>::new(0); 512])
        .expect("Out of memory");
    PagingRootEntry::new(Frame::containing(PhysAddr::new_truncate(
        target as *const _ as usize,
    )))
}

fn target_or_alloc<'a, PS>(
    entry: &mut PagingRawEntry<PS>,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> &'a mut [PagingRawEntry<PS::ReferenceTarget>]
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    if let Some(entry_reference) = entry.as_reference() {
        let target = unsafe {
            // SAFETY: trust in the process
            entry_reference.target_mut()
        };
        target
    } else if let Some(_) = entry.as_absent() {
        let target = allocator
            .alloc([PagingRawEntry::new(0); 512])
            .expect("Out of memory");
        let reference = PagingReferenceEntry::<PS>::new(Frame::containing(PhysAddr::new_truncate(
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
