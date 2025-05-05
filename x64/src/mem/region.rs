#[cfg(test)]
mod test;

use super::addr::Address;
use super::addr::PhysAddr;
use super::addr::VirtAddr;
use crate::mem::MemorySize;
use core::cmp::Ordering;
use core::fmt::Debug;
use core::fmt::Display;
use core::ops::Add;
use core::ops::AddAssign;
use core::ops::BitAnd;
use core::ops::BitAndAssign;

pub type PhysicalMemoryRegion = MemoryRegion<PhysAddr>;
pub type VirtualMemoryRegion = MemoryRegion<VirtAddr>;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion<S: const Address> {
    start: S,
    size: MemorySize,
}

pub struct ChunkIter<S: const Address> {
    current: S,
    region: MemoryRegion<S>,
    increment: MemorySize,
}

impl<S: const Address> MemoryRegion<S> {
    #[inline]
    pub const fn null() -> Self {
        Self {
            start: S::null(),
            size: MemorySize::zero(),
        }
    }
    #[inline]
    pub const fn new(start: S, size: MemorySize) -> Self {
        Self { start, size }
    }
    #[inline]
    pub fn new_boundaries(start: S, end: S) -> Self {
        Self {
            start,
            size: end - start,
        }
    }
}

impl<S: const Address> MemoryRegion<S> {
    #[inline]
    pub fn contains(&self, addr: S) -> bool {
        self.start() <= addr && addr < self.end()
    }
    #[inline]
    pub fn contains_start(&self, other: MemoryRegion<S>) -> bool {
        self.end() == other.start() || self.contains(other.start())
    }
    #[inline]
    pub fn contains_end(&self, other: MemoryRegion<S>) -> bool {
        self.end() == other.end() || self.contains(other.end())
    }
    #[inline]
    pub fn contains_region(&self, other: MemoryRegion<S>) -> bool {
        self.contains_start(other) && self.contains_end(other)
    }
    #[inline]
    pub const fn size(&self) -> MemorySize {
        self.size
    }
    #[inline]
    pub const fn start(&self) -> S {
        self.start
    }
    #[inline]
    pub const fn end(&self) -> S {
        self.start().add_truncate(self.size.as_usize())
    }
    #[inline]
    pub fn is_null(&self) -> bool {
        *self.size == 0
    }
}

impl<S: const Address> MemoryRegion<S> {
    pub fn union(&self, other: MemoryRegion<S>) -> Option<MemoryRegion<S>> {
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
    pub fn intersect(&self, other: MemoryRegion<S>) -> MemoryRegion<S> {
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

impl<S: const Address> MemoryRegion<S> {
    pub fn take_start(&mut self, amount: usize) -> S {
        let start = self.start();
        self.start += amount;
        self.size -= amount;
        if *self.size() == 0 {
            *self = MemoryRegion::null();
        }
        start
    }
}

impl<S: const Address> MemoryRegion<S> {
    #[inline]
    pub const fn chunks(&self, align: MemorySize, size: MemorySize) -> ChunkIter<S> {
        let start = S::new_panic(self.start.as_usize().next_multiple_of(align.as_usize()));
        ChunkIter {
            current: start,
            region: *self,
            increment: size,
        }
    }
}

impl<S: const Address> Default for MemoryRegion<S> {
    fn default() -> Self {
        Self::null()
    }
}

impl<S: const Address> PartialOrd for MemoryRegion<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<S: const Address> Ord for MemoryRegion<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

impl<S: const Address> Add for MemoryRegion<S> {
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}
impl<S: const Address> BitAnd for MemoryRegion<S> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersect(rhs)
    }
}

impl<S: const Address> AddAssign for MemoryRegion<S> {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.union(rhs).unwrap();
    }
}
impl<S: const Address> BitAndAssign for MemoryRegion<S> {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.intersect(rhs);
    }
}

impl<S: const Address> Display for MemoryRegion<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{} - {}", self.start(), self.end())
    }
}

impl<S: const Address> Debug for MemoryRegion<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "MemoryRegion({} - {})", self.start(), self.end())
    }
}

impl<S: const Address> Iterator for ChunkIter<S> {
    type Item = MemoryRegion<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.region.contains(self.current) && self.region.contains(self.current + self.increment)
        {
            let current_region = MemoryRegion::new(self.current, self.increment);
            self.current += self.increment;
            Some(current_region)
        } else {
            None
        }
    }
}
