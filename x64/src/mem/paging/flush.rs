use crate::mem::addr::Address;
use crate::mem::addr::VirtAddr;
use core::arch::asm;

pub trait FlushStrategy {
    fn flush(addr: VirtAddr);
}

#[derive(Clone, Copy)]
pub struct NoFlush;
#[derive(Clone, Copy)]
pub struct AddressFlush;
#[derive(Clone, Copy)]
pub struct GlobalFlush;

impl FlushStrategy for NoFlush {
    #[inline]
    fn flush(_: VirtAddr) {
        // NOOP
    }
}

impl FlushStrategy for AddressFlush {
    #[inline]
    fn flush(addr: VirtAddr) {
        unsafe {
            // # Safety
            // Nothing to worry about
            asm! {
                "invlpg [{addr}]",
                addr = in(reg) addr.as_usize()
            }
        }
    }
}

impl FlushStrategy for GlobalFlush {
    #[inline]
    fn flush(_: VirtAddr) {
        unsafe {
            // # Safety
            // Nothing to worry about
            asm! {
                "mov {scratch:r}, cr3",
                "mov cr3, {scratch:r}",
                scratch = in(reg) 0
            }
        }
    }
}
