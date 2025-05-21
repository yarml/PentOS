#[cfg(test)]
mod test;

use core::fmt::Debug;
use core::fmt::Display;
use core::iter::Sum;
use core::ops::Add;
use core::ops::AddAssign;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Mul;
use core::ops::MulAssign;
use core::ops::Sub;
use core::ops::SubAssign;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct MemorySize {
    inner: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryUnit {
    Byte,
    KibiByte,
    MebiByte,
    GibiByte,
    TebiByte,
    PebiByte,
    ExbiByte,
}

impl MemorySize {
    #[inline]
    pub const fn new(size: usize) -> Self {
        Self { inner: size }
    }
    #[inline]
    pub const fn zero() -> Self {
        Self { inner: 0 }
    }
}

impl MemorySize {
    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.inner
    }
}

impl MemoryUnit {
    const MEMORY_ORDERS: usize = 7;
    const MEMORY_UNITS: [char; Self::MEMORY_ORDERS] = ['B', 'K', 'M', 'G', 'T', 'P', 'E'];
    const MEMORY_SIZES: [usize; Self::MEMORY_ORDERS] = [
        1,
        1024,
        1024 * 1024,
        1024 * 1024 * 1024,
        1024 * 1024 * 1024 * 1024,
        1024 * 1024 * 1024 * 1024 * 1024,
        1024 * 1024 * 1024 * 1024 * 1024 * 1024,
    ];
}

impl MemoryUnit {
    #[inline]
    pub const fn from_order(order: usize) -> Option<Self> {
        match order {
            0 => Some(Self::Byte),
            1 => Some(Self::KibiByte),
            2 => Some(Self::MebiByte),
            3 => Some(Self::GibiByte),
            4 => Some(Self::TebiByte),
            5 => Some(Self::PebiByte),
            6 => Some(Self::ExbiByte),
            _ => None,
        }
    }
}

impl MemoryUnit {
    #[inline]
    pub const fn order(&self) -> usize {
        match self {
            MemoryUnit::Byte => 0,
            MemoryUnit::KibiByte => 1,
            MemoryUnit::MebiByte => 2,
            MemoryUnit::GibiByte => 3,
            MemoryUnit::TebiByte => 4,
            MemoryUnit::PebiByte => 5,
            MemoryUnit::ExbiByte => 6,
        }
    }
    #[inline]
    pub const fn suffix(&self) -> char {
        Self::MEMORY_UNITS[self.order()]
    }
    #[inline]
    pub const fn size(&self) -> usize {
        Self::MEMORY_SIZES[self.order()]
    }
    #[inline]
    pub const fn component(&self, size: usize) -> usize {
        (size / self.size()) % 1024
    }
}

impl Deref for MemorySize {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MemorySize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AddAssign for MemorySize {
    fn add_assign(&mut self, rhs: Self) {
        self.inner += rhs.inner;
    }
}
impl AddAssign<usize> for MemorySize {
    fn add_assign(&mut self, rhs: usize) {
        self.inner += rhs;
    }
}
impl SubAssign for MemorySize {
    fn sub_assign(&mut self, rhs: Self) {
        self.inner -= rhs.inner;
    }
}
impl SubAssign<usize> for MemorySize {
    fn sub_assign(&mut self, rhs: usize) {
        self.inner -= rhs;
    }
}
impl MulAssign<usize> for MemorySize {
    fn mul_assign(&mut self, rhs: usize) {
        self.inner *= rhs;
    }
}

impl Add for MemorySize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.inner + rhs.inner)
    }
}
impl Add<usize> for MemorySize {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.inner + rhs)
    }
}
impl Sub for MemorySize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.inner - rhs.inner)
    }
}
impl Sub<usize> for MemorySize {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.inner - rhs)
    }
}
impl Mul<usize> for MemorySize {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        Self::new(self.inner * rhs)
    }
}

impl Sum for MemorySize {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::new(iter.map(|s| *s).sum())
    }
}

impl Debug for MemorySize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Display for MemorySize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.inner == 0 {
            write!(f, "0{}", MemoryUnit::Byte.suffix())
        } else {
            for unit in (0..MemoryUnit::MEMORY_ORDERS)
                .rev()
                .filter_map(MemoryUnit::from_order)
                .filter(|unit| unit.component(self.inner) != 0)
            {
                write!(f, "{}{}", unit.component(self.inner), unit.suffix())?
            }
            Ok(())
        }
    }
}
