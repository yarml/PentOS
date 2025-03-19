#[cfg(test)]
mod test;

use crate::mem::MemorySize;
use crate::mem::addr::PhysAddr;
use core::cmp::Ordering;
use core::fmt::Debug;
use core::fmt::Display;
use core::ops::Add;
use core::ops::AddAssign;
use core::ops::BitAnd;
use core::ops::BitAndAssign;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    start: PhysAddr,
    size: MemorySize,
}

impl MemoryRegion {
    #[inline]
    pub const fn null() -> Self {
        Self {
            start: PhysAddr::null(),
            size: MemorySize::zero(),
        }
    }
    #[inline]
    pub const fn new(start: PhysAddr, size: MemorySize) -> Self {
        Self { start, size }
    }
    #[inline]
    pub fn new_boundaries(start: PhysAddr, end: PhysAddr) -> Self {
        Self {
            start,
            size: end - start,
        }
    }
}

impl MemoryRegion {
    #[inline]
    pub fn contains(&self, addr: PhysAddr) -> bool {
        self.start() <= addr && addr < self.end()
    }
    #[inline]
    pub fn contains_start(&self, other: MemoryRegion) -> bool {
        self.end() == other.start() || self.contains(other.start())
    }
    #[inline]
    pub fn contains_end(&self, other: MemoryRegion) -> bool {
        self.end() == other.end() || self.contains(other.end())
    }
    #[inline]
    pub fn contains_region(&self, other: MemoryRegion) -> bool {
        self.contains_start(other) && self.contains_end(other)
    }
    #[inline]
    pub const fn size(&self) -> MemorySize {
        self.size
    }
    #[inline]
    pub const fn start(&self) -> PhysAddr {
        self.start
    }
    #[inline]
    pub fn end(&self) -> PhysAddr {
        self.start().add_truncate(*self.size)
    }
    #[inline]
    pub fn is_null(&self) -> bool {
        *self.size == 0
    }
}

impl MemoryRegion {
    pub fn union(&self, other: MemoryRegion) -> Option<MemoryRegion> {
        if self.contains_region(other) {
            Some(*self)
        } else if other.contains_region(*self) {
            Some(other)
        } else if self.contains_start(other) {
            Some(MemoryRegion::new_boundaries(self.start(), other.end()))
        } else if self.contains_end(other) {
            Some(MemoryRegion::new_boundaries(other.start(), self.end()))
        } else {
            None
        }
    }
    pub fn intersect(&self, other: MemoryRegion) -> MemoryRegion {
        if self.contains_region(other) {
            other
        } else if other.contains_region(*self) {
            *self
        } else if self.contains_start(other) {
            MemoryRegion::new_boundaries(other.start(), self.end())
        } else if self.contains_end(other) {
            MemoryRegion::new_boundaries(self.start(), other.end())
        } else {
            MemoryRegion::null()
        }
    }
}

impl MemoryRegion {
    pub fn take_start(&mut self, amount: usize) -> PhysAddr {
        let start = self.start();
        self.start += amount;
        self.size -= amount;
        if *self.size() == 0 {
            *self = MemoryRegion::null();
        }
        start
    }
}

impl Default for MemoryRegion {
    fn default() -> Self {
        Self::null()
    }
}

impl PartialOrd for MemoryRegion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for MemoryRegion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl Add for MemoryRegion {
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}
impl BitAnd for MemoryRegion {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersect(rhs)
    }
}

impl AddAssign for MemoryRegion {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.union(rhs).unwrap();
    }
}
impl BitAndAssign for MemoryRegion {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.intersect(rhs);
    }
}

impl Display for MemoryRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} - {}", self.start(), self.end())
    }
}

impl Debug for MemoryRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "MemoryRegion({} - {})", self.start(), self.end())
    }
}
