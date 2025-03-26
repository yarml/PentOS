use crate::define_addr;
use crate::mem::page::Page;
use crate::mem::page::size::PageSize;

define_addr!(VirtAddr, make_canonical);

impl VirtAddr {
    #[inline]
    pub const fn page<S: PageSize>(&self) -> Page<S> {
        Page::containing(*self)
    }
}

#[inline]
const fn make_canonical(addr: usize) -> usize {
    if addr & 0x0000800000000000 == 0 {
        addr & 0x0000FFFFFFFFFFFF
    } else {
        addr | 0xFFFF000000000000
    }
}
