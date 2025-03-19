use crate::define_addr;

const VIRT_MASK: usize = 0x0000_FFFF_FFFF_FFFF;

define_addr!(VirtAddr, VIRT_MASK);
