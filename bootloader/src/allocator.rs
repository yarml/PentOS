use crate::mmap::MemoryMap;
use common::mem::addr::PhysAddr;
use core::mem;
use log::debug;

/// Post boot services allocator
pub struct Allocator<const MAX: usize> {
    mmap: MemoryMap<MAX>,
}

impl<const MAX: usize> Allocator<MAX> {
    /// # Safety
    /// Caller must make sure all memory under 1M is not included in the memory map
    /// as well as LOADER_CODE and LOADER_DATA regions.
    pub unsafe fn init(mut mmap: MemoryMap<MAX>) -> Self {
        mmap.minimize();
        mmap.sort_size();
        Self { mmap }
    }

    pub fn fini<const LOADER_MAX: usize>(
        self,
        loader_mmap: MemoryMap<LOADER_MAX>,
    ) -> MemoryMap<MAX> {
        let Self { mut mmap } = self;
        for region in loader_mmap.iter() {
            mmap.add(*region);
        }
        mmap.minimize();
        mmap.sort_start_addr();
        mmap
    }
}

impl<const MAX: usize> Allocator<MAX> {
    pub fn print(&self) {
        for region in self.mmap.iter() {
            debug!(
                "{:?} {pg_count} pages",
                region,
                pg_count = *region.size() / 4096
            );
        }
    }

    pub fn alloc_raw(&mut self, size: usize) -> Option<PhysAddr> {
        let size = size.next_multiple_of(4096);
        for region in self.mmap.iter_mut() {
            if *region.size() >= size {
                let start = region.take_start(size);
                // Ya we sort twice on each allocations, tu as un problème?
                self.mmap.minimize();
                self.mmap.sort_size();
                return Some(start);
            }
        }
        None
    }
    pub fn alloc<T>(&mut self, init: T) -> Option<&'static mut T> {
        if mem::align_of::<T>() > 4096 || !mem::align_of::<T>().is_power_of_two() {
            return None;
        }
        let size = core::mem::size_of::<T>();
        let start = self.alloc_raw(size)?;
        let ptr = start.as_mut_ptr::<T>();
        unsafe {
            // SAFETY: `start` is (over)aligned, and cannot be null since we dismiss all memory below 1M
            ptr.write(init);
            Some(&mut *ptr)
        }
    }
}
