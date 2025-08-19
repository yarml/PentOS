use core::sync::atomic::{AtomicUsize, Ordering};

use x64::mem::addr::{Address, VirtAddr};

static INFO_AREA: AtomicUsize = AtomicUsize::new(0xfffffff000000000);

pub fn allocate_info_space(size: usize) -> VirtAddr {
    let size = size.next_multiple_of(0x1000);
    let addr = INFO_AREA.fetch_add(size, Ordering::Relaxed);
    if addr + size > 0xfffffffc00000000 {
        panic!("Out of memory for info area");
    }

    VirtAddr::new_panic(addr)
}
