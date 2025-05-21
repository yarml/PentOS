pub mod size;

use super::addr::Address;
use crate::mem::addr::VirtAddr;
use core::fmt::Debug;
use core::fmt::Display;
use core::marker::PhantomData;
use core::ops::Add;
use size::PageSize;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Page<S: PageSize> {
    boundary: VirtAddr,
    _phantom: PhantomData<S>,
}

impl<S: PageSize> Page<S> {
    #[inline]
    pub const fn containing(addr: VirtAddr) -> Self {
        Self {
            boundary: VirtAddr::new_panic(addr.as_usize() & S::MASK),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn from_number(num: usize) -> Self {
        Self {
            boundary: VirtAddr::new_panic(num << S::SHIFT),
            _phantom: PhantomData,
        }
    }
}

impl<S: PageSize> Page<S> {
    #[inline]
    pub const fn boundary(&self) -> VirtAddr {
        self.boundary
    }

    #[inline]
    pub const fn number(&self) -> usize {
        self.boundary.as_usize() >> S::SHIFT
    }
    #[inline]
    pub const fn order_index<OtherSize: PageSize>(&self) -> usize {
        (self.boundary().as_usize() >> OtherSize::SHIFT) & 0x1FF
    }
}

impl<S: PageSize> Add<usize> for Page<S> {
    type Output = Page<S>;

    fn add(self, rhs: usize) -> Self::Output {
        Self::from_number(self.number() + rhs)
    }
}

impl<S: PageSize> Debug for Page<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Page{}#{}@{}", S::SIZE, self.number(), self.boundary())
    }
}

impl<S: PageSize> Display for Page<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Page{}#{}", S::SIZE, self.number())
    }
}
