use {
    crate::allocator::PostBootAllocator,
    system::pat::pat_index,
    x64::{
        mem::{
            addr::{Address, PhysAddr},
            frame::Frame,
            page::size::{Page512GiB, PageSize, PageSizeMap},
            paging::{PagingMapEntry, PagingRawEntry, PagingReferenceEntry, PagingRootEntry},
        },
        msr::pat::MemoryType as PatMemoryType,
    },
};

pub fn paging_root_new<const ALLOCATOR_CAP: usize>(
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> PagingRootEntry {
    let target = allocator
        .alloc([PagingRawEntry::<Page512GiB>::new(0); 512])
        .expect("Out of memory");
    PagingRootEntry::new(Frame::containing(PhysAddr::new_panic(
        target as *const _ as usize,
    )))
}

pub fn page_target_or_new<'a, PS, const ALLOCATOR_CAP: usize>(
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

pub fn page_map_new<PS: PageSizeMap>(
    frame: Frame<PS::PhysicalPageSize>,
    write: bool,
    exec: bool,
    mtype: PatMemoryType,
) -> PagingRawEntry<PS> {
    let mut new_entry = PagingMapEntry::new(frame).with_pat_index(pat_index(mtype));

    if write {
        new_entry = new_entry.write();
    }

    if exec {
        new_entry = new_entry.exec();
    }
    new_entry.to_raw()
}
