mod midmem;

use core::{
    alloc::{Allocator, GlobalAlloc, Layout},
    ptr::NonNull,
};

pub use midmem::MidMemKalloc;

#[global_allocator]
static KALLOC: Kalloc = Kalloc;
static MIDMEM_KALLOC: MidMemKalloc = MidMemKalloc;

struct Kalloc;

/// TODO: use allocators other than MidMemKalloc
unsafe impl GlobalAlloc for Kalloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        MIDMEM_KALLOC
            .allocate(layout)
            .map(|non_null| non_null.as_mut_ptr())
            .unwrap_or_default()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            // SAFETY: guarenteed by caller
            MIDMEM_KALLOC.deallocate(NonNull::new_unchecked(ptr), layout)
        }
    }
}
