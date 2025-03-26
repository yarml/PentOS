use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use crate::define_addr;
use crate::mem::frame::Frame;
use crate::mem::frame::size::FrameSize;

use super::virt::VirtAddr;

const PHYS_MASK: usize = 0x00FF_FFFF_FFFF_FFFF;

static PHYSICAL_MEMORY_OFFSET: AtomicUsize = AtomicUsize::new(0);

define_addr!(PhysAddr, make_canonical);

impl PhysAddr {
    #[inline]
    pub fn set_memory_offset(offset: usize) {
        PHYSICAL_MEMORY_OFFSET.store(offset, Ordering::Relaxed);
    }
}

impl PhysAddr {
    #[inline]
    pub const fn frame<S: FrameSize>(&self) -> Frame<S> {
        Frame::containing(*self)
    }

    #[inline]
    pub fn to_virt(&self) -> VirtAddr {
        VirtAddr::new_truncate(self.inner + PHYSICAL_MEMORY_OFFSET.load(Ordering::Relaxed))
    }
}

#[inline]
const fn make_canonical(addr: usize) -> usize {
    addr & PHYS_MASK
}
