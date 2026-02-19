pub mod size;

use {
    crate::mem::phys::lowmem::size::{LowMemFrame64KiB, LowMemFrame128KiB, LowMemFrameSize},
    utils::collections::smallvec::SmallVec,
    x64::mem::frame::{Frame, FrameRange, size::Frame4KiB},
};

#[derive(Default)]
pub struct LowMemAllocator {
    free64k: SmallVec<u8, 240>,
    free128k: SmallVec<u8, 120>,
}

impl LowMemAllocator {
    pub const fn new() -> Self {
        Self {
            free64k: SmallVec::new(),
            free128k: SmallVec::new(),
        }
    }
}

impl LowMemAllocator {
    pub fn alloc(&mut self, size: LowMemFrameSize) -> Option<FrameRange<Frame4KiB>> {
        match size {
            LowMemFrameSize::K64 => self.alloc_64k().map(|extent| extent.into_range()),
            LowMemFrameSize::K128 => self.alloc_128k().map(|extent| extent.into_range()),
        }
    }
    pub fn free(&mut self, frames: FrameRange<Frame4KiB>, size: LowMemFrameSize) {
        match size {
            LowMemFrameSize::K64 => self.free_64k(frames.into_frame()),
            LowMemFrameSize::K128 => self.free_128k(frames.into_frame()),
        }
    }

    pub fn alloc_128k(&mut self) -> Option<LowMemFrame128KiB> {
        self.free128k
            .pop()
            .map(|index| Frame::from_number(index as usize))
    }
    pub fn alloc_64k(&mut self) -> Option<LowMemFrame64KiB> {
        if let Some(frame) = self
            .free64k
            .pop()
            .map(|index| Frame::from_number(index as usize))
        {
            return Some(frame);
        }

        let k128frame_index = self.alloc_128k()?.number() as u8;
        let alloc_result_index = k128frame_index * 2;
        let storeaway_index = alloc_result_index + 1;

        unsafe {
            // SAFETY: free64k contains enough space for all 64k frames free
            self.free64k.push(storeaway_index).unwrap_unchecked()
        };

        Some(Frame::from_number(alloc_result_index as usize))
    }

    pub fn free_128k(&mut self, frames: LowMemFrame128KiB) {
        let index = frames.number() as u8;

        if self.free128k.contains(&index) {
            panic!("LowMem: Double free");
        }

        unsafe {
            // SAFETY: free64k contains enough space for all 64k frames free
            self.free128k.push(index).unwrap_unchecked();
        }
    }
    pub fn free_64k(&mut self, frames: LowMemFrame64KiB) {
        let index = frames.number() as u8;
        let buddy_index = index ^ 1;
        let parent_index = index / 2;

        // FIXME: This contains, followed by erase_value, does a double run of the array, but man, i don't wanna fix that now
        if self.free64k.contains(&index) {
            panic!("LowMem: Double free");
        }

        if self.free64k.erase_value(buddy_index).is_none() {
            unsafe {
                // SAFETY: free64k contains enough space for all 64k frames free
                self.free64k.push(index).unwrap_unchecked();
            }
        } else {
            unsafe {
                // SAFETY: free64k contains enough space for all 64k frames free
                self.free128k.push(parent_index).unwrap_unchecked();
            }
        }
    }
}
