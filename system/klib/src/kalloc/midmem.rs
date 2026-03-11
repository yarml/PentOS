//! Kernel heap allocator backed by [`crate::mem::phys::MIDMEM_ALLOCATOR`].
//!
//! # Design
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                      MidMemKalloc                       │
//! │                                                         │
//! │  buckets[0] (8 B)   -> slab -> slab -> ...              │
//! │  buckets[1] (16 B)  -> slab -> slab -> ...              │
//! │  ...                                                    │
//! │  buckets[7] (1 KiB) -> slab -> slab -> ...              │
//! │                                                         │
//! │  allocations > 1 KiB --> LargeHeader + data pages       │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Small allocations (≤ 1 KiB)
//!
//! Requests are rounded up to the nearest [`BucketSize`] and served from a
//! per-size list of 4 KiB *slabs*.  Each slab carries a [`slab::SlabHeader`] at
//! its base and an intrusive free-list through its slots.  Full slabs are
//! segregated from partial slabs so allocation never walks past full ones.
//! Completely empty slabs are returned to the page allocator immediately.
//!
//! ## Large allocations (> 1 KiB)
//!
//! A dedicated header page is allocated immediately before the data region.
//! Deallocation identifies the header by a magic word and frees both the
//! header page and the data pages in one call to the page allocator.
//!
//! ## Thread safety
//!
//! The whole allocator is protected by a lock.
//!

mod bucket;
mod large;
pub mod size;
mod slab;

use {
    bucket::Bucket,
    core::{
        alloc::{AllocError, Allocator, Layout},
        mem,
        ptr::NonNull,
    },
    large::LargeHeader,
    size::BucketSize,
    spinlocks::mutex::{SpinMutex, SpinMutexGuard},
    x64::mem::frame::size::{Frame4KiB, FrameSize},
};

static BUCKETS: SpinMutex<[Bucket; BucketSize::COUNT]> = SpinMutex::new([
    Bucket::new(BucketSize::B8),
    Bucket::new(BucketSize::B16),
    Bucket::new(BucketSize::B32),
    Bucket::new(BucketSize::B64),
    Bucket::new(BucketSize::B128),
    Bucket::new(BucketSize::B256),
    Bucket::new(BucketSize::B512),
    Bucket::new(BucketSize::B1K),
]);

pub struct MidMemKalloc;

unsafe impl Allocator for MidMemKalloc {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        Self::alloc_inner(layout).ok_or(AllocError)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { Self::dealloc_inner(ptr, layout) };
    }
}

impl MidMemKalloc {
    fn alloc_inner(layout: Layout) -> Option<NonNull<[u8]>> {
        let size = effective_size(layout);

        if size <= BucketSize::MAX_SLAB_ALLOC {
            Self::alloc_small(size)
        } else {
            Self::alloc_large(layout)
        }
    }

    unsafe fn dealloc_inner(ptr: NonNull<u8>, layout: Layout) {
        let size = effective_size(layout);

        if size <= BucketSize::MAX_SLAB_ALLOC {
            unsafe { Self::dealloc_small(ptr) };
        } else {
            unsafe { LargeHeader::dealloc(ptr) };
        }
    }

    fn alloc_small(size: usize) -> Option<NonNull<[u8]>> {
        let bucket_size = BucketSize::fit(size)?;
        let mut buckets = get_buckets();
        let ptr = buckets[bucket_size.index()].alloc()?;
        Some(NonNull::slice_from_raw_parts(ptr, bucket_size as usize))
    }

    /// # Safety
    /// `ptr` must have been returned by `alloc_small`.
    unsafe fn dealloc_small(ptr: NonNull<u8>) {
        let mut buckets = get_buckets();
        let slab = unsafe { &*slab::SlabHeader::from_slot_ptr(ptr) };
        let bucket_size = slab.bucket_size;
        unsafe { buckets[bucket_size.index()].dealloc(ptr) };
    }

    fn alloc_large(layout: Layout) -> Option<NonNull<[u8]>> {
        let size = effective_size(layout);
        let data_pages = size.div_ceil(Frame4KiB::SIZE);
        let ptr = LargeHeader::alloc(data_pages)?;
        Some(NonNull::slice_from_raw_parts(ptr, size))
    }
}

#[inline]
fn effective_size(layout: Layout) -> usize {
    layout
        .size()
        .max(layout.align())
        .max(mem::size_of::<*mut u8>())
}

#[inline]
fn get_buckets() -> SpinMutexGuard<'static, [Bucket; BucketSize::COUNT]> {
    BUCKETS.lock()
}
