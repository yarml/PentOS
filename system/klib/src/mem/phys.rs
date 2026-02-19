use {
    crate::{
        bootinfo::bootinfo,
        mem::phys::{
            lowmem::{LowMemAllocator, size::LowMemFrame64KiB},
            midmem::MidMemAllocator,
        },
    },
    config::{
        pmem::{LOWMEM, MIDMEM},
        vmem::PHYSICAL_MAPPING_REGION,
    },
    spinlocks::mutex::Mutex,
    x64::mem::{
        addr::{Address, PhysAddr},
        frame::{
            Frame, FrameRange,
            size::{Frame4KiB, FrameSize},
        },
    },
};

pub mod lowmem;
pub mod midmem;

pub static LOWMEM_ALLOCATOR: Mutex<LowMemAllocator> = Mutex::new(LowMemAllocator::new());
pub static MIDMEM_ALLOCATOR: MidMemAllocator = MidMemAllocator::zero();

/// # Safety
/// Should be called once in the BSP and no other allocator method should be called before this initialization ends
pub unsafe fn init() {
    let bootinfo = bootinfo();
    let mmap = &bootinfo.mmap[..bootinfo.mmap_len];

    PhysAddr::set_memory_offset(PHYSICAL_MAPPING_REGION.start().as_usize());

    unsafe {
        // SAFETY: guarenteed by caller
        MIDMEM_ALLOCATOR.init()
    };

    let mut lowmem_allocator = LOWMEM_ALLOCATOR.lock();
    for &(mut entry) in mmap {
        if LOWMEM.contains_region(entry) {
            entry.take_start(LowMemFrame64KiB::SIZE - *entry.start() % LowMemFrame64KiB::SIZE);
            entry.take_end(*entry.end() % LowMemFrame64KiB::SIZE);

            // TODO: dealloc by larger frames, not just 64K.
            while *entry.size() >= LowMemFrame64KiB::SIZE {
                lowmem_allocator.free_64k(Frame::containing(entry.start()));
                entry.take_start(LowMemFrame64KiB::SIZE);
            }
        }
        if MIDMEM.contains_region(entry) {
            // TODO: dealloc by larger frames, not just 4K.
            // Works for now tho.
            while *entry.size() >= Frame4KiB::SIZE {
                MIDMEM_ALLOCATOR.dealloc(FrameRange::new(Frame::containing(entry.start()), 1));
                entry.take_start(Frame4KiB::SIZE);
            }
        }
        // TODO: HIGHMEM
    }
}
