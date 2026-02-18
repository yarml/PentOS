use {
    crate::mem::phys::{MIDMEM_ALLOCATOR, midmem::MidFrameSize},
    common::collections::smallvec::SmallVec,
    config::{
        kalloc::MIDMEM_KALLOC_FREELIST_CAP,
        vmem::{KERNELSPACE, PHYSICAL_MAPPING_REGION},
    },
    core::{
        alloc::{AllocError, Allocator, Layout},
        ptr::{self, NonNull},
    },
    spinlocks::mutex::Mutex,
    x64::mem::{
        addr::{Address, PhysAddr},
        frame::{
            Frame, FrameRange,
            size::{Frame2MiB, Frame4KiB, FrameSize},
        },
    },
};

// static BUCKET_FREELIST: Mutex<SmallVec<&'static mut Bucket, MIDMEM_KALLOC_FREELIST_CAP>> =
//     Mutex::new(SmallVec::new());

pub struct MidMemKalloc;

// struct Bucket {
//     size: MidFrameSize,
// }

unsafe impl Allocator for MidMemKalloc {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let framesize = layout_to_framesize(layout).ok_or(AllocError)?;
        let frame_range = MIDMEM_ALLOCATOR.alloc(framesize).ok_or(AllocError)?;
        let start_ptr = frame_range.start().boundary().to_virt().as_mut_ptr::<u8>();
        let ptr = ptr::slice_from_raw_parts_mut(start_ptr, frame_range.size().as_usize());

        Ok(unsafe {
            // SAFETY: frame_range guarenteed to be valid
            NonNull::new_unchecked(ptr)
        })
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let ptr = ptr.as_ptr() as usize - PHYSICAL_MAPPING_REGION.start().as_usize();
        let framesize = layout_to_framesize(layout).unwrap();
        let size = framesize.size();
        let pg_count = size / Frame4KiB::SIZE;

        let frame_range = FrameRange::new(Frame::containing(PhysAddr::new_panic(ptr)), pg_count);

        MIDMEM_ALLOCATOR.dealloc(frame_range)
    }
}

fn layout_to_framesize(layout: Layout) -> Option<MidFrameSize> {
    let size = layout.size();

    if size <= Frame4KiB::SIZE {
        Some(MidFrameSize::K4)
    } else if size <= 16 * Frame4KiB::SIZE {
        Some(MidFrameSize::K64)
    } else if size <= 32 * Frame4KiB::SIZE {
        Some(MidFrameSize::K128)
    } else if size <= Frame2MiB::SIZE {
        Some(MidFrameSize::M2)
    } else if size <= 4 * Frame2MiB::SIZE {
        Some(MidFrameSize::M8)
    } else {
        None
    }
}
