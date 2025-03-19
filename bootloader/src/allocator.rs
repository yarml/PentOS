use crate::mmap::MemoryMap;
use common::mem::addr::PhysAddr;
use log::debug;

/// Post boot services allocator
pub struct Allocator<const MAX: usize> {
    mmap: MemoryMap<MAX>,
}

impl<const MAX: usize> Allocator<MAX> {
    pub fn init(mut mmap: MemoryMap<MAX>) -> Self {
        mmap.minimize();
        mmap.sort_size();
        Self { mmap }
    }

    pub fn disable<const LOADER_MAX: usize>(
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

    pub fn alloc(&mut self, size: usize) -> Option<PhysAddr> {
        let size = size.next_multiple_of(4096);
        for region in self.mmap.iter_mut() {
            if *region.size() >= size {
                let start = region.take_start(size);
                self.mmap.minimize();
                self.mmap.sort_size();
                return Some(start);
            }
        }
        None
    }
}
