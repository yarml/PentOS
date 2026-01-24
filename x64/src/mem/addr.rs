#[cfg(test)]
mod test;

mod phys;
mod virt;

use {
    super::MemorySize,
    core::{
        fmt::{Debug, Display},
        ops::{Add, AddAssign, DerefMut, Sub, SubAssign},
    },
};

pub use {phys::PhysAddr, virt::VirtAddr};

pub const trait Address:
    Clone
    + Copy
    + DerefMut<Target = usize>
    + From<usize>
    + From<u64>
    + AddAssign<usize>
    + AddAssign<MemorySize>
    + SubAssign<usize>
    + SubAssign<MemorySize>
    + Add<usize, Output = Self>
    + Add<MemorySize, Output = Self>
    + Sub<usize, Output = Self>
    + Sub<MemorySize, Output = Self>
    + Sub<Self, Output = MemorySize>
    + Eq
    + Ord
    + Debug
    + Display
{
    fn null() -> Self;
    fn new(addr: usize) -> Option<Self>;
    fn new_truncate(addr: usize) -> Self;
    /// # Safety
    /// Caller must ensure the provided addr is valid for the implementing type
    unsafe fn new_unchecked(addr: usize) -> Self;
    fn new_panic(addr: usize) -> Self;

    fn add(&self, offset: usize) -> Option<Self>;
    fn add_truncate(&self, offset: usize) -> Self;
    fn add_panic(&self, offset: usize) -> Self;
    fn sub_truncate(&self, offset: usize) -> Self;
    fn is_null(&self) -> bool;

    fn as_usize(&self) -> usize;
    fn as_u64(&self) -> u64;
    fn as_ptr<T>(&self) -> *const T;
    fn as_mut_ptr<T>(&self) -> *mut T;
}

/// Implementation detail
#[doc(hidden)]
#[macro_export]
macro_rules! define_addr {
    ($name:ident, $make_canonical:expr) => {
        use {
            core::{
                fmt::{Debug, Display},
                ops::{Add, AddAssign, Deref, DerefMut, Sub, SubAssign},
            },
            $crate::mem::{MemorySize, addr::Address},
        };

        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name {
            inner: usize,
        }

        impl $name {
            pub const MAX: Self = Self::new_truncate(usize::MAX);
            pub const MIN: Self = Self::null();
        }

        impl const Address for $name {
            // Constructors
            #[inline(always)]
            fn null() -> Self {
                Self { inner: 0 }
            }
            #[inline(always)]
            fn new(addr: usize) -> Option<Self> {
                if addr == $make_canonical(addr) {
                    Some(Self { inner: addr })
                } else {
                    None
                }
            }
            #[inline(always)]
            fn new_truncate(addr: usize) -> Self {
                Self {
                    inner: $make_canonical(addr),
                }
            }
            #[inline(always)]
            unsafe fn new_unchecked(addr: usize) -> Self {
                Self { inner: addr }
            }
            #[inline(always)]
            fn new_panic(addr: usize) -> Self {
                if let Some(addr) = Self::new(addr) {
                    addr
                } else {
                    panic!("Invalid address")
                }
            }

            // Operations
            #[inline(always)]
            fn add(&self, offset: usize) -> Option<Self> {
                Self::new(self.inner + offset)
            }
            #[inline(always)]
            fn add_truncate(&self, offset: usize) -> Self {
                Self::new_truncate(self.inner + offset)
            }
            #[inline(always)]
            fn add_panic(&self, offset: usize) -> Self {
                Self::new_panic(self.inner + offset)
            }
            #[inline(always)]
            fn sub_truncate(&self, offset: usize) -> Self {
                Self::new_panic(self.inner - offset)
            }

            #[inline(always)]
            fn is_null(&self) -> bool {
                self.inner == 0
            }

            // Casts
            #[inline(always)]
            fn as_usize(&self) -> usize {
                self.inner
            }
            #[inline(always)]
            fn as_u64(&self) -> u64 {
                self.inner as u64
            }
            #[inline(always)]
            fn as_ptr<T>(&self) -> *const T {
                self.inner as *const T
            }
            #[inline(always)]
            fn as_mut_ptr<T>(&self) -> *mut T {
                self.inner as *mut T
            }
        }

        impl Deref for $name {
            type Target = usize;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl DerefMut for $name {
            #[inline(always)]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }

        impl From<usize> for $name {
            #[inline(always)]
            fn from(value: usize) -> Self {
                Self::new_panic(value)
            }
        }

        impl From<u64> for $name {
            #[inline(always)]
            fn from(value: u64) -> Self {
                Self::new_panic(value as usize)
            }
        }

        impl<T: ?Sized> From<*const T> for $name {
            #[inline(always)]
            fn from(value: *const T) -> Self {
                Self::new_panic(value as *const () as usize)
            }
        }

        impl<T: ?Sized> From<*mut T> for $name {
            #[inline(always)]
            fn from(value: *mut T) -> Self {
                Self::new_panic(value as *const () as usize)
            }
        }

        impl From<$name> for usize {
            #[inline(always)]
            fn from(value: $name) -> Self {
                value.as_usize()
            }
        }

        impl From<$name> for u64 {
            #[inline(always)]
            fn from(value: $name) -> Self {
                value.as_u64()
            }
        }

        impl AddAssign<usize> for $name {
            #[inline(always)]
            fn add_assign(&mut self, rhs: usize) {
                *self = self.add_truncate(rhs);
            }
        }
        impl AddAssign<MemorySize> for $name {
            #[inline(always)]
            fn add_assign(&mut self, rhs: MemorySize) {
                *self += *rhs;
            }
        }
        impl SubAssign<usize> for $name {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: usize) {
                *self = self.sub_truncate(rhs);
            }
        }
        impl SubAssign<MemorySize> for $name {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: MemorySize) {
                *self -= *rhs;
            }
        }
        impl Add<usize> for $name {
            type Output = Self;

            #[inline(always)]
            fn add(self, rhs: usize) -> Self::Output {
                self.add_truncate(rhs)
            }
        }
        impl Add<MemorySize> for $name {
            type Output = Self;

            #[inline(always)]
            fn add(self, rhs: MemorySize) -> Self::Output {
                self.add_truncate(*rhs)
            }
        }
        impl Sub<usize> for $name {
            type Output = Self;

            #[inline(always)]
            fn sub(self, rhs: usize) -> Self::Output {
                self.sub_truncate(rhs)
            }
        }
        impl Sub<MemorySize> for $name {
            type Output = Self;

            #[inline(always)]
            fn sub(self, rhs: MemorySize) -> Self::Output {
                self.sub_truncate(*rhs)
            }
        }
        impl Sub for $name {
            type Output = MemorySize;

            #[inline(always)]
            fn sub(self, rhs: Self) -> Self::Output {
                MemorySize::new(self.inner - rhs.inner)
            }
        }

        impl Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({:016x})", stringify!($name), self.inner)
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{:016x}", self.inner)
            }
        }
    };
}
