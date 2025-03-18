#[cfg(test)]
mod test;

use crate::mem::MemorySize;
use crate::mem::frame::Frame;
use crate::mem::frame::size::FrameSize;
use core::fmt::Debug;
use core::fmt::Display;
use core::ops::Add;
use core::ops::AddAssign;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Sub;
use core::ops::SubAssign;

macro_rules! phys_truncated {
    ($addr:expr) => {
        $addr & 0x00FFFFFFFFFFFFFF
    };
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr {
    inner: usize,
}

impl PhysAddr {
    pub const MAX: Self = Self::new_truncate(usize::MAX);
    pub const MIN: Self = Self::null();

    #[inline]
    pub const fn new(addr: usize) -> Option<Self> {
        if addr == phys_truncated!(addr) {
            Some(Self { inner: addr })
        } else {
            None
        }
    }
    #[inline]
    pub const fn new_truncate(addr: usize) -> Self {
        Self {
            inner: phys_truncated!(addr),
        }
    }
    #[inline]
    pub const fn null() -> Self {
        Self { inner: 0 }
    }
}

impl PhysAddr {
    #[inline]
    pub const fn add(&self, offset: usize) -> Option<Self> {
        Self::new(self.inner + offset)
    }
    #[inline]
    pub const fn add_truncate(&self, offset: usize) -> Self {
        Self::new_truncate(self.inner + offset)
    }
    #[inline]
    pub const fn sub_truncate(&self, offset: usize) -> Self {
        Self::new_truncate(self.inner - offset)
    }

    #[inline]
    pub const fn frame<S: FrameSize>(&self) -> Frame<S> {
        Frame::containing(self)
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.inner == 0
    }
}

impl PhysAddr {
    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.inner
    }
    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.inner as u64
    }
}

impl Deref for PhysAddr {
    type Target = usize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PhysAddr {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<usize> for PhysAddr {
    #[inline]
    fn from(value: usize) -> Self {
        Self::new_truncate(value)
    }
}

impl From<u64> for PhysAddr {
    #[inline]
    fn from(value: u64) -> Self {
        Self::new_truncate(value as usize)
    }
}

impl From<PhysAddr> for usize {
    #[inline]
    fn from(value: PhysAddr) -> Self {
        value.as_usize()
    }
}

impl From<PhysAddr> for u64 {
    #[inline]
    fn from(value: PhysAddr) -> Self {
        value.as_u64()
    }
}

impl AddAssign<usize> for PhysAddr {
    #[inline]
    fn add_assign(&mut self, rhs: usize) {
        *self = self.add_truncate(rhs);
    }
}
impl AddAssign<MemorySize> for PhysAddr {
    #[inline]
    fn add_assign(&mut self, rhs: MemorySize) {
        *self += *rhs;
    }
}
impl SubAssign<usize> for PhysAddr {
    #[inline]
    fn sub_assign(&mut self, rhs: usize) {
        *self = self.sub_truncate(rhs);
    }
}
impl SubAssign<MemorySize> for PhysAddr {
    #[inline]
    fn sub_assign(&mut self, rhs: MemorySize) {
        *self -= *rhs;
    }
}
impl Add<usize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        self.add_truncate(rhs)
    }
}
impl Add<MemorySize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: MemorySize) -> Self::Output {
        self.add_truncate(*rhs)
    }
}
impl Sub<usize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self::Output {
        self.sub_truncate(rhs)
    }
}
impl Sub<MemorySize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: MemorySize) -> Self::Output {
        self.sub_truncate(*rhs)
    }
}
impl Sub for PhysAddr {
    type Output = MemorySize;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        MemorySize::new(self.inner - rhs.inner)
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PhysAddr({:016x})", self.inner)
    }
}

impl Display for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:016x}", self.inner)
    }
}
