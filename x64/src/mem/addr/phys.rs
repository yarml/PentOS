use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use crate::define_addr;
use crate::mem::frame::Frame;
use crate::mem::frame::size::FrameSize;

use super::virt::VirtAddr;

const PHYS_MASK: usize = 0x00FF_FFFF_FFFF_FFFF;

// Bootloader keeps this at 0, since it always has identity mapping until the
// last moment when it doesn't need direct access anymore. Kernel sets this to
// the offset it finds in BootInfo as soon as it gets control.
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

    #[inline]
    /// # Safety
    /// Must ensure that physical memory offset is set and valid
    /// and that the memory location contains a valid instance of T
    /// and that the memory location is not mutably aliased
    pub unsafe fn to_ref<'a, T>(&self) -> &'a T {
        unsafe {
            // SAFETY: ensured by caller
            &*self.to_virt().as_ptr()
        }
    }

    #[inline]
    /// # Safety
    /// Must ensure that physical memory offset is set and valid
    /// and that the memory location contains a valid instance of T
    /// and that the memory location is not aliased
    pub unsafe fn to_mut<'a, T>(&self) -> &'a mut T {
        unsafe {
            // SAFETY: ensured by caller
            &mut *self.to_virt().as_mut_ptr()
        }
    }
}

#[inline]
const fn make_canonical(addr: usize) -> usize {
    addr & PHYS_MASK
}
