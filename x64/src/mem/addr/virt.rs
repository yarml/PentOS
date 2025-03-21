use crate::define_addr;

define_addr!(VirtAddr, make_canonical);

#[inline]
const fn make_canonical(addr: usize) -> usize {
    if addr & 0x0000800000000000 == 0 {
        addr & 0x0000FFFFFFFFFFFF
    } else {
        addr | 0xFFFF000000000000
    }
}
