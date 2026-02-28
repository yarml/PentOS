mod block;
mod freelist;
mod size;

use {
    crate::mem::phys::midmem::block::Block,
    core::{
        cell::UnsafeCell,
        hint,
        sync::atomic::{AtomicU8, Ordering},
    },
    x64::mem::{
        addr::{Address, PhysAddr},
        frame::{FrameRange, size::Frame4KiB},
    },
};

pub use size::MidFrameSize;

const BLOCK_SIZE: usize = 512 * 1024 * 1024;

pub struct MidMemAllocator {
    lock: AtomicU8,
    blocks: [UnsafeCell<Block>; 8],
}

/// # Safety
/// We use an internal lock
unsafe impl Sync for MidMemAllocator {}

impl MidMemAllocator {
    pub const fn zero() -> Self {
        Self {
            lock: AtomicU8::new(0),
            blocks: [
                UnsafeCell::new(Block::zero()),
                UnsafeCell::new(Block::zero()),
                UnsafeCell::new(Block::zero()),
                UnsafeCell::new(Block::zero()),
                UnsafeCell::new(Block::zero()),
                UnsafeCell::new(Block::zero()),
                UnsafeCell::new(Block::zero()),
                UnsafeCell::new(Block::zero()),
            ],
        }
    }
}

impl MidMemAllocator {
    /// # Safety
    /// Should be called once in the BSP and no other allocator
    /// method should be called before this initialization ends
    pub unsafe fn init(&self) {
        for (i, block) in self.blocks.iter().enumerate() {
            unsafe {
                // SAFETY: Guarenteed by caller
                (&mut *block.get()).init(PhysAddr::new_panic(i * BLOCK_SIZE));
            }
        }
    }
}

impl MidMemAllocator {
    pub fn alloc(&self, size: MidFrameSize) -> Option<FrameRange<Frame4KiB>> {
        let index = self.lock_any_block();

        let block = unsafe {
            // SAFETY: acquired da lock
            self.blocks[index].as_mut_unchecked()
        };
        // debug!("MidMemAllocator::alloc: -> Block#{index}");
        let result = block.alloc(size);

        self.unlock_block(index);

        if result.is_some() {
            return result;
        }

        self.alloc_insist(size, index)
    }

    fn alloc_insist(&self, size: MidFrameSize, skip: usize) -> Option<FrameRange<Frame4KiB>> {
        for index in (0..8).filter(|i| *i != skip) {
            self.lock_block(index);
            let block = unsafe {
                // SAFETY: acquired da lock
                self.blocks[index].as_mut_unchecked()
            };
            // debug!("MidMemAllocator::alloc: -> Block#{index}");
            let result = block.alloc(size);
            self.unlock_block(index);

            if result.is_some() {
                return result;
            }
        }

        None
    }

    pub fn dealloc(&self, frame: FrameRange<Frame4KiB>) {
        let block_index = *frame.start().boundary() / BLOCK_SIZE;
        // debug!("MidMemAllocator::dealloc Block#{block_index}");
        self.lock_block(block_index);

        let block = unsafe {
            // SAFETY: acquired lock
            self.blocks[block_index].as_mut_unchecked()
        };
        block.dealloc(frame);

        self.unlock_block(block_index);
    }

    fn lock_block(&self, index: usize) {
        while self
            .lock
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                (lock & 1 << index == 0).then_some(lock | 1 << index)
            })
            .is_err()
        {
            hint::spin_loop();
        }
    }
    fn unlock_block(&self, index: usize) {
        self.lock.fetch_and(!(1 << index), Ordering::Release);
    }
    fn lock_any_block(&self) -> usize {
        loop {
            if let Ok(prev_lock) =
                self.lock
                    .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                        let free_block_index = lock.trailing_ones();
                        (free_block_index != 8).then_some(lock | 1 << free_block_index)
                    })
            {
                break prev_lock;
            }
            hint::spin_loop();
        }
        .trailing_ones() as usize
    }
}

#[cfg(feature = "test")]
pub mod test_exports {
    pub const BLOCK_SIZE: usize = super::BLOCK_SIZE;
    pub mod block {
        pub use super::super::block::*;
    }
    pub mod freelist {
        pub use super::super::freelist::*;
    }
    pub mod size {
        pub use super::super::size::*;
    }
}
