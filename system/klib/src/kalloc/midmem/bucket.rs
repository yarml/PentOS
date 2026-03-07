//! A [`Bucket`] owns a linked list of slabs all serving the same
//! [`BucketSize`].  It is the allocation/deallocation entry point for
//! small objects.
//!
//! ## Slab list organisation
//!
//! ```text
//! partial_head → [slab with free slots] → [slab with free slots] → null
//! full_head    → [full slab] → [full slab] → null
//! ```
//!
//! We keep a separate **partial list** and **full list** so that allocation
//! never has to walk full slabs.  When a partial slab becomes full it
//! migrates to the full list; when a full slab gets a slot back it migrates
//! to the partial list.

use {
    super::{size::BucketSize, slab::SlabHeader},
    crate::mem::phys::MIDMEM_ALLOCATOR,
    config::vmem::PHYSICAL_MAPPING_REGION,
    core::ptr::NonNull,
    x64::mem::{addr::Address, frame::FrameRange},
};

pub struct Bucket {
    size: BucketSize,
    partial_head: *mut SlabHeader,
    full_head: *mut SlabHeader,
}

/// # Safety
/// Technically, this is not safe, but the only place we use this Bucket is within a Mutex, and should be fine???
unsafe impl Send for Bucket {}

impl Bucket {
    pub const fn new(size: BucketSize) -> Self {
        Self {
            size,
            partial_head: core::ptr::null_mut(),
            full_head: core::ptr::null_mut(),
        }
    }
}

impl Bucket {
    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        if let Some(ptr) = self.alloc_from_partial() {
            return Some(ptr);
        }
        let slab = self.grow()?;
        let ptr =
            unsafe { (*slab).alloc_slot() }.expect("newly initialised slab must have free slots");

        if unsafe { (*slab).is_full() } {
            self.push_full(slab);
        } else {
            self.push_partial(slab);
        }

        Some(ptr)
    }

    /// # Safety
    /// `slot` must have been returned by [`Bucket::alloc`] on the same bucket.
    pub unsafe fn dealloc(&mut self, slot: NonNull<u8>) {
        let slab = unsafe { &mut *SlabHeader::from_slot_ptr(slot) };
        let was_full = slab.is_full();

        unsafe { slab.dealloc_slot(slot) };

        if was_full {
            self.remove_from_full(slab as *mut SlabHeader);
            self.push_partial(slab as *mut SlabHeader);
        } else if slab.is_empty() {
            self.remove_from_partial(slab as *mut SlabHeader);
            self.release_slab(slab as *mut SlabHeader);
        }
    }
}

impl Bucket {
    fn alloc_from_partial(&mut self) -> Option<NonNull<u8>> {
        let slab = unsafe { self.partial_head.as_mut()? };
        let ptr = slab.alloc_slot()?;

        if slab.is_full() {
            let slab_ptr = slab as *mut SlabHeader;
            self.remove_from_partial(slab_ptr);
            self.push_full(slab_ptr);
        }

        Some(ptr)
    }

    fn grow(&mut self) -> Option<*mut SlabHeader> {
        let frame_range = MIDMEM_ALLOCATOR.alloc(crate::mem::phys::midmem::MidFrameSize::K4)?;

        let virt = frame_range.start().boundary().to_virt();
        Some(unsafe { SlabHeader::init(virt, self.size) })
    }

    fn release_slab(&mut self, slab: *mut SlabHeader) {
        use x64::mem::{
            addr::PhysAddr,
            frame::{Frame, size::Frame4KiB},
        };

        let virt_addr = slab as usize;
        let phys_addr = virt_addr - PHYSICAL_MAPPING_REGION.start().as_usize();
        let frame = Frame::<Frame4KiB>::containing(PhysAddr::new_panic(phys_addr));
        let frame_range = FrameRange::new(frame, 1);
        MIDMEM_ALLOCATOR.dealloc(frame_range);
    }

    fn push_partial(&mut self, slab: *mut SlabHeader) {
        unsafe { (*slab).next = self.partial_head };
        self.partial_head = slab;
    }

    fn push_full(&mut self, slab: *mut SlabHeader) {
        unsafe { (*slab).next = self.full_head };
        self.full_head = slab;
    }

    fn remove_from_partial(&mut self, target: *mut SlabHeader) {
        Self::remove_from_list(&mut self.partial_head, target);
    }

    fn remove_from_full(&mut self, target: *mut SlabHeader) {
        Self::remove_from_list(&mut self.full_head, target);
    }

    fn remove_from_list(head: &mut *mut SlabHeader, target: *mut SlabHeader) {
        unsafe {
            let mut cursor: *mut *mut SlabHeader = head as *mut *mut SlabHeader;
            loop {
                let node = *cursor;
                if node.is_null() {
                    break;
                }
                if node == target {
                    *cursor = (*node).next;
                    (*node).next = core::ptr::null_mut();
                    break;
                }
                cursor = &mut (*node).next as *mut *mut SlabHeader;
            }
        }
    }
}
