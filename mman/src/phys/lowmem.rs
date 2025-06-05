use common::collections::smallvec::SmallVec;
use spinlocks::mutex::Mutex;
use x64::mem::frame::Frame;
use x64::mem::frame::size::Frame64KiB;
use x64::mem::frame::size::Frame128KiB;

#[derive(Default)]
pub struct LowMemAllocator {
    free64k: Mutex<SmallVec<u8, 240>>,
    free128k: Mutex<SmallVec<u8, 120>>,
}

impl LowMemAllocator {
    pub const fn new() -> Self {
        Self {
            free64k: Mutex::new(SmallVec::new()),
            free128k: Mutex::new(SmallVec::new()),
        }
    }
}

impl LowMemAllocator {
    pub fn alloc128k(&self) -> Option<Frame<Frame128KiB>> {
        let index = {
            let mut lock = self.free128k.lock();
            lock.pop()
        }?;
        Some(Frame::from_number(index as usize))
    }

    pub fn alloc64k(&self) -> Option<Frame<Frame64KiB>> {
        let mut lock = self.free64k.lock();
        let index = lock.pop();

        if let Some(index) = index {
            return Some(Frame::from_number(index as usize));
        }

        let frame128k = self.alloc128k()?;

        unsafe {
            // SAFETY: free64k was just empty, and has been locked the entire time
            lock.push(frame128k.number() as u8 * 2).unwrap_unchecked()
        };
        Some(Frame::from_number(frame128k.number() * 2 + 1))
    }

    pub fn free128k(&self, frame: Frame<Frame128KiB>) {
        let mut lock = self.free128k.lock();
        lock.push(frame.number() as u8).unwrap();
    }
    pub fn free64k(&self, frame: Frame<Frame64KiB>) {
        // Look if this frame's buddy is also in the free list, if so, remove it and add a free128k
        // Otherwise, add this to the freelist and that's it
        let mut lock = self.free64k.lock();

        let Some(buddy_index) = lock
            .iter()
            .enumerate()
            .find(|(_, frame_index)| **frame_index / 2 == frame.number() as u8 / 2)
            .map(|(i, _)| i)
        else {
            lock.push(frame.number() as u8).unwrap();
            return;
        };
        lock.erase(buddy_index);
        self.free128k(Frame::from_number(frame.number() / 2));
    }
}
