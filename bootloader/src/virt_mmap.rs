use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use boot_protocol::BootInfo;
use core::mem;
use uefi::boot::MemoryType;
use uefi::mem::memory_map::MemoryMap;
use uefi::mem::memory_map::MemoryMapOwned;
use x64::mem::MemoryRegion;
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
use x64::msr::pat::MemoryType as PatMemoryType;
use x64::msr::pat::pat_index;

pub struct VirtMemMap {}

pub fn map(
    root: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    frame: Frame<Frame4KiB>,
    page: Page<Page4KiB>,
    write: bool,
    exec: bool,
    mtype: PatMemoryType,
) {
    let pml4_table = unsafe { root.target_mut() };

    let pdp_table = target_or_alloc(
        pml4_table[page.order_index::<Page512GiB>()].as_raw(),
        allocator,
    );
    let pd_table = target_or_alloc(&mut pdp_table[page.order_index::<Page1GiB>()], allocator);
    let pt_table = target_or_alloc(&mut pd_table[page.order_index::<Page2MiB>()], allocator);
    let pt_entry = &mut pt_table[page.order_index::<Page4KiB>()];

    let mut target = PagingMapEntry::<Page4KiB>::new(frame);

    if write {
        target = target.write();
    }
    if exec {
        target = target.exec();
    }

    target = target.with_pat_index(pat_index(mtype));

    *pt_entry = target.to_raw();
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

pub fn identity_and_offset_mapping(
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
    mmap: &MemoryMapOwned,
    offset: usize,
) -> PagingRootEntry {
    let root_map = new_root(allocator);
    // TODO: Setup virtual memory for kernel

    for entry in mmap.entries() {
        let region = MemoryRegion::new(
            PhysAddr::new_truncate(entry.phys_start as usize),
            MemorySize::new(entry.page_count as usize * 4096),
        );
        if entry.phys_start < 1024 * 1024
            || (entry.ty != MemoryType::CONVENTIONAL
                && entry.ty != MemoryType::LOADER_CODE
                && entry.ty != MemoryType::LOADER_DATA
                && entry.ty != MemoryType::BOOT_SERVICES_CODE
                && entry.ty != MemoryType::BOOT_SERVICES_DATA)
        {
            continue;
        }

        let exec = entry.ty == MemoryType::LOADER_CODE;

        let pg_count = *region.size() / 4096;
        for i in 0..pg_count {
            let frame = Frame::containing(region.start() + i * 4096);
            let identity_vaddr = region.start().to_virt() + i * 4096;
            let offset_vadr = identity_vaddr + offset;
            let page = Page::containing(identity_vaddr);
            let offset_page = Page::containing(offset_vadr);
            map(
                root_map,
                allocator,
                frame,
                page,
                true,
                exec,
                PatMemoryType::WriteBack,
            );
            map(
                root_map,
                allocator,
                frame,
                offset_page,
                true,
                false,
                PatMemoryType::WriteBack,
            );
        }
    }

    root_map
}

pub fn map_bootinfo(
    bootinfo: &BootInfo,
    target: Page<Page4KiB>,
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) {
    let bootinfo = Frame::containing(PhysAddr::new_truncate(bootinfo as *const _ as usize));
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
