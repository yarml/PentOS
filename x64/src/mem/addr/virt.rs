use crate::define_addr;
use crate::mem::page::Page;
use crate::mem::page::size::PageSize;

define_addr!(VirtAddr, make_canonical);

impl VirtAddr {
    #[inline]
    pub const fn page<S: PageSize>(&self) -> Page<S> {
        Page::containing(*self)
    }

    #[inline]
    /// # Safety
    /// Must ensure that the memory location contains a valid instance of T
    /// and that the memory location is not mutably aliased
    pub unsafe fn to_ref<'a, T>(&self) -> &'a T {
        unsafe {
            // SAFETY: ensured by caller
            &*self.as_ptr()
        }
    }

    #[inline]
    /// # Safety
    /// Must ensure that the memory location contains a valid instance of T
    /// and that the memory location is not aliased
    pub unsafe fn to_mut<'a, T>(&self) -> &'a mut T {
        unsafe {
            // SAFETY: ensured by caller
            &mut *self.as_mut_ptr()
        }
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
