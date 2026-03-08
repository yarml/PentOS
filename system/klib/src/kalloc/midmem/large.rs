//! Allocations larger than [`super::BucketSize::MAX_SLAB_ALLOC`] are handled by
//! allocating directly from the page allocator and prepending a
//! [`LargeHeader`] to the returned memory.
//!
//! Memory layout
//! ```text
//! ┌──────────────────────────────┐  <- page boundary
//! │  LargeHeader  (4K page)      │  <- magic + page count
//! ├──────────────────────────────┤  <- page boundary (+4K)
//! │  user data                   │
//! │  ...                         │
//! └──────────────────────────────┘
//! ```
//!
//! The header is kept on its own dedicated page so that the user data region
//! starts at a page boundary, which simplifies alignment and means the
//! allocator never needs to embed metadata inside the data region.
//!
//! On [`dealloc`](super::MidMemKalloc) we step back one page from the data
//! pointer, verify the magic, read the page count, and return
//! `1 + page_count` pages to the page allocator.

use {
    crate::mem::phys::{MIDMEM_ALLOCATOR, midmem::MidFrameSize},
    system::vmem::PHYSICAL_MAPPING_REGION,
    core::ptr::NonNull,
    x64::mem::{
        addr::{Address, PhysAddr, VirtAddr},
        frame::{
            Frame, FrameRange,
            size::{Frame4KiB, FrameSize},
        },
    },
};

const MAGIC: u64 = 0xDEAD_BEEF_CAFE_F00D;

#[repr(C)]
pub struct LargeHeader {
    magic: u64,
    /// not counting the header page
    page_count: usize,
}

impl LargeHeader {
    pub fn alloc(data_pages: usize) -> Option<NonNull<u8>> {
        let total_pages = data_pages + 1;
        let frame_size = pages_to_midframe_size(total_pages)?;
        let actual_pages = frame_size.size() / Frame4KiB::SIZE;

        let range = MIDMEM_ALLOCATOR.alloc(frame_size)?;
        let header_virt: VirtAddr = range.start().boundary().to_virt();

        unsafe {
            header_virt.as_mut_ptr::<LargeHeader>().write(LargeHeader {
                magic: MAGIC,
                page_count: actual_pages - 1,
            });
        }

        let data_virt = VirtAddr::new_panic(header_virt.as_usize() + Frame4KiB::SIZE);
        Some(unsafe { NonNull::new_unchecked(data_virt.as_mut_ptr::<u8>()) })
    }

    /// # Safety
    /// `ptr` must have been returned by [`LargeHeader::alloc`] and must not
    /// have been freed before.
    ///
    /// # Panics
    /// Panics if the magic word is wrong (double-free or wild pointer).
    pub unsafe fn dealloc(ptr: NonNull<u8>) {
        let header_virt = ptr.as_ptr() as usize - Frame4KiB::SIZE;
        let header = unsafe { &*(header_virt as *const LargeHeader) };

        assert_eq!(
            header.magic, MAGIC,
            "MidMemKalloc: large dealloc magic mismatch"
        );

        let total_pages = header.page_count + 1;
        let frame_size =
            pages_to_midframe_size(total_pages).expect("LargeHeader stored invalid page count");

        let phys = header_virt - PHYSICAL_MAPPING_REGION.start().as_usize();
        let frame = Frame::<Frame4KiB>::containing(PhysAddr::new_panic(phys));
        let range = FrameRange::new(frame, frame_size.k4_count());
        MIDMEM_ALLOCATOR.dealloc(range);
    }
}

fn pages_to_midframe_size(pages: usize) -> Option<MidFrameSize> {
    MidFrameSize::ALL
        .iter()
        .copied()
        .find(|s| s.k4_count() >= pages)
}

impl MidFrameSize {
    const ALL: [MidFrameSize; 5] = [
        MidFrameSize::K4,
        MidFrameSize::K64,
        MidFrameSize::K128,
        MidFrameSize::M2,
        MidFrameSize::M8,
    ];
}
