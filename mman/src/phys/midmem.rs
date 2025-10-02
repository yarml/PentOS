mod block;
mod freelist;
mod size;

use {
    crate::phys::midmem::{block::Block, size::MidFrameSize},
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

const BLOCK_SIZE: usize = 512 * 1024 * 1024;

pub struct MidMemAllocator {
    lock: AtomicU8,
    blocks: [UnsafeCell<Block>; 8],
}

impl MidMemAllocator {
    #[allow(clippy::erasing_op)]
    #[allow(clippy::identity_op)]
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            lock: AtomicU8::new(0),
            blocks: [
                UnsafeCell::new(Block::new(PhysAddr::new_panic(0 * BLOCK_SIZE))),
                UnsafeCell::new(Block::new(PhysAddr::new_panic(1 * BLOCK_SIZE))),
                UnsafeCell::new(Block::new(PhysAddr::new_panic(2 * BLOCK_SIZE))),
                UnsafeCell::new(Block::new(PhysAddr::new_panic(3 * BLOCK_SIZE))),
                UnsafeCell::new(Block::new(PhysAddr::new_panic(4 * BLOCK_SIZE))),
                UnsafeCell::new(Block::new(PhysAddr::new_panic(5 * BLOCK_SIZE))),
                UnsafeCell::new(Block::new(PhysAddr::new_panic(6 * BLOCK_SIZE))),
                UnsafeCell::new(Block::new(PhysAddr::new_panic(7 * BLOCK_SIZE))),
            ],
        }
    }
}

impl MidMemAllocator {
    pub fn alloc(&self, size: MidFrameSize) -> Option<FrameRange<Frame4KiB>> {
        let index = loop {
            if let Ok(prev_lock) =
                self.lock
                    .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                        let free_block_index = lock.trailing_ones();
                        if free_block_index == 8 {
                            None
                        } else {
                            Some(lock | 1 << free_block_index)
                        }
                    })
            {
                break prev_lock;
            }
            hint::spin_loop();
        }
        .trailing_ones() as usize;

        let block = unsafe {
            // SAFETY: acquired da lock
            self.blocks[index].as_mut_unchecked()
        };
        let result = block.alloc(size);

        self.lock.fetch_and(!(1 << index), Ordering::Release);

        result
    }

    pub fn dealloc(&self, frame: FrameRange<Frame4KiB>) {
        let block_index = *frame.start().boundary() / BLOCK_SIZE;

        while self
            .lock
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |lock| {
                if lock & 1 << block_index != 0 {
                    None
                } else {
                    Some(lock | 1 << block_index)
                }
            })
            .is_err()
        {
            hint::spin_loop();
        }

        let block = unsafe {
            // SAFETY: acquired lock
            self.blocks[block_index].as_mut_unchecked()
        };
        block.dealloc(frame);

        self.lock.fetch_and(!(1 << block_index), Ordering::Release);
    }
}
