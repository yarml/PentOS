use x64::mem::paging::{PagingRawEntry, PagingRootEntry};

pub mod map;
pub mod phys;

/// # Safety
/// Should be called once in the BSP and no other allocator method should be called before this initialization ends
pub(crate) unsafe fn init() {
    unsafe {
        // SAFETY: guarenteed by caller
        phys::init();
    }

    let map_root = PagingRootEntry::current();
    let map = unsafe {
        // SAFETY. Guarenteed by bootloader
        map_root.target_mut()
    };
    map[0..256].fill(PagingRawEntry::new(0));
}
