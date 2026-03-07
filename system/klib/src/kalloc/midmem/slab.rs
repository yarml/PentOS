//! A *slab* is exactly one 4 KiB page managed by a single [`BucketSize`].
//!
//! Memory layout of a slab page
//! ```text
//! ┌─────────────────────────┐  <- page boundary (4 KiB aligned)
//! │  SlabHeader             │
//! ├─────────────────────────┤
//! │  slot 0                 │  <- size = BucketSize bytes
//! │  slot 1                 │
//! │  …                      │
//! │  slot N-1               │
//! └─────────────────────────┘
//! ```
//!
//! Free slots form an *intrusive singly-linked list* through their first
//! pointer-sized word.  While a slot is free its first `usize` is the
//! virtual address of the next free slot (or 0 for end-of-list).
//!
//! The [`SlabHeader`] is placed at the very start of the page so that given
//! any pointer inside the page we can recover the header with a single
//! aligned mask:
//!
//! ```rs
//! header = ptr & !(PAGE_SIZE - 1)
//! ```

use {
    super::size::BucketSize,
    core::{mem, ptr::NonNull},
    x64::mem::{
        addr::{Address, VirtAddr},
        frame::size::{Frame4KiB, FrameSize},
    },
};

/// Stored at the very beginning of every slab page.
///
/// # Invariants
/// * `free_head` either points into this slab's slot area or is null.
/// * `free_count` number of slots reachable via `free_head`.
/// * `next` links slabs within the same [`super::bucket::Bucket`].
#[repr(C)]
pub struct SlabHeader {
    pub next: *mut SlabHeader,
    free_head: *mut u8,
    pub free_count: usize,
    pub bucket_size: BucketSize,
}

impl SlabHeader {
    pub unsafe fn init(page: VirtAddr, bucket: BucketSize) -> *mut Self {
        let header_ptr = page.as_mut_ptr::<SlabHeader>();

        let slot_size = bucket as usize;
        let header_size = mem::size_of::<SlabHeader>();

        let slots_start = (page.as_usize() + header_size).next_multiple_of(slot_size);
        let slots_end = page.as_usize() + Frame4KiB::SIZE;
        let count = (slots_end - slots_start) / slot_size;

        let mut prev: *mut u8 = core::ptr::null_mut();
        for i in (0..count).rev() {
            let slot = (slots_start + i * slot_size) as *mut *mut u8;
            unsafe { slot.write(prev) };
            prev = slot as *mut u8;
        }

        unsafe {
            header_ptr.write(SlabHeader {
                next: core::ptr::null_mut(),
                free_head: prev,
                free_count: count,
                bucket_size: bucket,
            });
        }

        header_ptr
    }

    #[inline]
    pub fn alloc_slot(&mut self) -> Option<NonNull<u8>> {
        if self.free_head.is_null() {
            return None;
        }
        let slot = self.free_head;
        self.free_head = unsafe { *(slot as *const *mut u8) };
        self.free_count -= 1;
        Some(unsafe { NonNull::new_unchecked(slot) })
    }

    #[inline]
    pub unsafe fn dealloc_slot(&mut self, slot: NonNull<u8>) {
        unsafe { (slot.as_ptr() as *mut *mut u8).write(self.free_head) };
        self.free_head = slot.as_ptr();
        self.free_count += 1;
    }

    #[inline]
    pub unsafe fn from_slot_ptr(ptr: NonNull<u8>) -> *mut SlabHeader {
        let page_base = ptr.as_ptr() as usize & !(Frame4KiB::SIZE - 1);
        page_base as *mut SlabHeader
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.free_count == self.bucket_size.slots_per_slab()
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.free_count == 0
    }
}
