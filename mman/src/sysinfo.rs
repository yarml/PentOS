use config::vmem::SYSINFO_REGION;
use core::alloc::AllocError;
use core::alloc::Allocator;
use core::alloc::Layout;
use core::ptr::NonNull;
use core::slice;
use log::error;
use x64::mem::addr::Address;
use x64::mem::addr::VirtAddr;

pub struct SysinfoAllocator {
    bump: VirtAddr,
}

impl SysinfoAllocator {
    const fn new() -> Self {
        Self {
            bump: SYSINFO_REGION.start(),
        }
    }
}

unsafe impl Allocator for SysinfoAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let start = VirtAddr::new(self.bump.next_multiple_of(layout.align())).ok_or(AllocError)?;
        let end = start + layout.size();
        if !SYSINFO_REGION.contains(end) {
            error!("Could not allocate sysinfo: {layout:?}");
            return Err(AllocError);
        }

        // TODO: allocate physical memory
        // TODO: synchronize mapping

        let result = unsafe {
            // # Safety
            // We just allocated fresh physical memory, and just mapped it
            // Cannot go wrong
            slice::from_raw_parts(start.as_ptr(), layout.size())
        };
        Ok(NonNull::from(result))
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _: Layout) {
        if !SYSINFO_REGION.contains(ptr.as_ptr().into()) {
            panic!("Attempt to deallocate non sysinfo memory with sysinfo allocator")
        }
        // NOOP
    }
}
