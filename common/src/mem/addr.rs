#[cfg(test)]
mod test;

mod phys;
mod virt;

pub use phys::PhysAddr;
pub use virt::VirtAddr;

/// Implementation detail
#[doc(hidden)]
#[macro_export]
macro_rules! define_addr {
    ($name:ident, $mask:expr) => {
        use core::fmt::Debug;
        use core::fmt::Display;
        use core::ops::Add;
        use core::ops::AddAssign;
        use core::ops::Deref;
        use core::ops::DerefMut;
        use core::ops::Sub;
        use core::ops::SubAssign;
        use $crate::mem::MemorySize;

        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name {
            inner: usize,
        }

        impl $name {
            pub const MAX: Self = Self::new_truncate(usize::MAX);
            pub const MIN: Self = Self::null();

            #[inline]
            pub const fn new(addr: usize) -> Option<Self> {
                if addr == addr & $mask {
                    Some(Self { inner: addr })
                } else {
                    None
                }
            }
            #[inline]
            pub const fn new_truncate(addr: usize) -> Self {
                Self {
                    inner: addr & $mask,
                }
            }
            #[inline]
            pub const fn null() -> Self {
                Self { inner: 0 }
            }
        }

        impl $name {
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
            pub const fn is_null(&self) -> bool {
                self.inner == 0
            }
        }

        impl $name {
            #[inline]
            pub const fn as_usize(&self) -> usize {
                self.inner
            }
            #[inline]
            pub const fn as_u64(&self) -> u64 {
                self.inner as u64
            }
            #[inline]
            pub const fn as_ptr<T>(&self) -> *const T {
                self.inner as *const T
            }
            #[inline]
            pub const fn as_mut_ptr<T>(&self) -> *mut T {
                self.inner as *mut T
            }
        }

        impl Deref for $name {
            type Target = usize;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl DerefMut for $name {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }

        impl From<usize> for $name {
            #[inline]
            fn from(value: usize) -> Self {
                Self::new_truncate(value)
            }
        }

        impl From<u64> for $name {
            #[inline]
            fn from(value: u64) -> Self {
                Self::new_truncate(value as usize)
            }
        }

        impl<T> From<*const T> for $name {
            #[inline]
            fn from(value: *const T) -> Self {
                Self::new_truncate(value as usize)
            }
        }

        impl<T> From<*mut T> for $name {
            #[inline]
            fn from(value: *mut T) -> Self {
                Self::new_truncate(value as usize)
            }
        }

        impl From<$name> for usize {
            #[inline]
            fn from(value: $name) -> Self {
                value.as_usize()
            }
        }

        impl From<$name> for u64 {
            #[inline]
            fn from(value: $name) -> Self {
                value.as_u64()
            }
        }

        impl AddAssign<usize> for $name {
            #[inline]
            fn add_assign(&mut self, rhs: usize) {
                *self = self.add_truncate(rhs);
            }
        }
        impl AddAssign<MemorySize> for $name {
            #[inline]
            fn add_assign(&mut self, rhs: MemorySize) {
                *self += *rhs;
            }
        }
        impl SubAssign<usize> for $name {
            #[inline]
            fn sub_assign(&mut self, rhs: usize) {
                *self = self.sub_truncate(rhs);
            }
        }
        impl SubAssign<MemorySize> for $name {
            #[inline]
            fn sub_assign(&mut self, rhs: MemorySize) {
                *self -= *rhs;
            }
        }
        impl Add<usize> for $name {
            type Output = Self;

            #[inline]
            fn add(self, rhs: usize) -> Self::Output {
                self.add_truncate(rhs)
            }
        }
        impl Add<MemorySize> for $name {
            type Output = Self;

            #[inline]
            fn add(self, rhs: MemorySize) -> Self::Output {
                self.add_truncate(*rhs)
            }
        }
        impl Sub<usize> for $name {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: usize) -> Self::Output {
                self.sub_truncate(rhs)
            }
        }
        impl Sub<MemorySize> for $name {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: MemorySize) -> Self::Output {
                self.sub_truncate(*rhs)
            }
        }
        impl Sub for $name {
            type Output = MemorySize;

            #[inline]
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
