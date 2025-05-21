use crate::phys_mmap::PhysMemMap;
use core::cell::Cell;
use core::mem;
use log::debug;
use mman::phys::PhysicalAllocationRequest;
use mman::phys::PhysicalMemoryAllocator;
use x64::mem::MemorySize;
use x64::mem::PhysicalMemoryRegion;
use x64::mem::addr::Address;

pub const ALLOCATOR_CAP: usize = 256;

/// Post boot services allocator
pub struct PostBootAllocator<const MAX: usize> {
    mmap: Cell<PhysMemMap<MAX>>,
}

impl<const MAX: usize> PostBootAllocator<MAX> {
    /// # Safety
    /// Caller must make sure all memory under 1M is not included in the memory map
    /// as well as LOADER_CODE and LOADER_DATA regions.
    pub unsafe fn init(mut mmap: PhysMemMap<MAX>) -> Self {
        mmap.minimize();
        mmap.sort_size();
        Self {
            mmap: Cell::new(mmap),
        }
    }

    pub fn fini<const LOADER_MAX: usize>(
        self,
        loader_mmap: PhysMemMap<LOADER_MAX>,
    ) -> PhysMemMap<MAX> {
        let Self { mut mmap } = self;
        let mmap_mut = mmap.get_mut();
        for region in loader_mmap.iter() {
            mmap_mut.add(*region);
        }
        mmap_mut.minimize();
        mmap_mut.sort_start_addr();
        mmap.into_inner()
    }
}

impl<const MAX: usize> PostBootAllocator<MAX> {
    pub fn alloc<'a, T>(&mut self, init: T) -> Option<&'a mut T> {
        let size = MemorySize::new(mem::size_of::<T>());
        let align = MemorySize::new(mem::align_of::<T>());
        let region = self.maybe_allocate(PhysicalAllocationRequest::size_align(size, align));
        let region = region?;
        let ptr = region.start().as_mut_ptr::<T>();
        unsafe {
            // SAFETY: `start` is (over)aligned, and cannot be null since we dismiss all memory below 1M
            ptr.write(init);
            Some(&mut *ptr)
        }
    }
}

impl<const MAX: usize> PhysicalMemoryAllocator for PostBootAllocator<MAX> {
    fn maybe_allocate(&self, req: PhysicalAllocationRequest) -> Option<PhysicalMemoryRegion> {
        let size = MemorySize::new(req.size.next_multiple_of(4096));
        let align = MemorySize::new(
            req.alignment
                .unwrap_or(MemorySize::new(2))
                .next_multiple_of(4096),
        );
        let chunk_size = MemorySize::new(size.next_multiple_of(*align));
        // Ignore req.continuous, the bootloader allocator always gives out continuous allocations
        if req.below.is_some() {
            unimplemented!(
                "Bootloader physical memory allocator does not support address contraint"
            );
        }

        let mut mmap = self.mmap.replace(PhysMemMap::new());

        let (region, chunk) = mmap
            .iter_mut()
            .enumerate()
            .flat_map(|(region_index, region)| {
                region
                    .chunks(align, chunk_size)
                    .next()
                    .map(move |chunk| (region_index, chunk))
            })
            .next()?;
        mmap[region].take_start(size);
        mmap.minimize();
        mmap.sort_size();

        self.mmap.replace(mmap);

        Some(PhysicalMemoryRegion::new(chunk.start(), size))
    }

    fn free(&self, _region: PhysicalMemoryRegion) {
        debug!("Freeing memory in bootloader stage, does that ever happen?");
        todo!()
    }
}
