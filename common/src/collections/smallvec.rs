use core::{
    borrow::{Borrow, BorrowMut},
    fmt::Debug,
    hash::Hash,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr,
    slice::{Iter, IterMut, SliceIndex},
};

/// A vector that contains all its element with its allocation unit
/// The size of the SmallVec depends on its capacity which cannot be changed
/// and has to be known at compile time
#[repr(C)]
pub struct SmallVec<T, const N: usize> {
    len: usize,
    buffer: [MaybeUninit<T>; N],
}

#[repr(C)]
pub struct SmallVecBuf<T> {
    len: usize,
    buffer: [MaybeUninit<T>],
}

impl<T, const N: usize> SmallVec<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [const { MaybeUninit::zeroed() }; N],
            len: 0,
        }
    }
}

impl<T, const N: usize> SmallVec<T, N> {
    pub const CAPACITY: usize = N;

    #[must_use = "check that value was added, otherwise it will just drop"]
    pub fn push(&mut self, value: T) -> Result<&T, T> {
        unsafe { common_push(&mut self.buffer, &mut self.len, N, value) }
    }
    pub fn pop(&mut self) -> Option<T> {
        unsafe { common_pop(&self.buffer, &mut self.len) }
    }

    pub fn erase(&mut self, index: usize) -> Option<T> {
        unsafe { common_erase(&mut self.buffer, &mut self.len, index) }
    }
    pub fn erase_value<U>(&mut self, val: U) -> Option<T>
    where
        T: PartialEq,
        U: Borrow<T>,
    {
        unsafe { common_erase_value(&mut self.buffer, &mut self.len, val) }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub const fn is_full(&self) -> bool {
        self.len == self.capacity()
    }
    pub const fn capacity(&self) -> usize {
        Self::CAPACITY
    }
}

impl<T> SmallVecBuf<T> {
    #[must_use = "check that value was added, otherwise it will just drop"]
    pub fn push(&mut self, value: T) -> Result<&T, T> {
        let cap = self.capacity();
        unsafe { common_push(&mut self.buffer, &mut self.len, cap, value) }
    }
    pub fn pop(&mut self) -> Option<T> {
        unsafe { common_pop(&self.buffer, &mut self.len) }
    }
    pub fn erase(&mut self, index: usize) -> Option<T> {
        unsafe { common_erase(&mut self.buffer, &mut self.len, index) }
    }
    pub fn erase_value<U>(&mut self, val: U) -> Option<T>
    where
        T: PartialEq,
        U: Borrow<T>,
    {
        unsafe { common_erase_value(&mut self.buffer, &mut self.len, val) }
    }

    pub const fn len(&self) -> usize {
        self.len
    }
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub const fn is_full(&self) -> bool {
        self.len == self.capacity()
    }
    pub const fn capacity(&self) -> usize {
        ptr::metadata(self)
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a SmallVec<T, N> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T, const N: usize> IntoIterator for &'a mut SmallVec<T, N> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, const N: usize> Drop for SmallVec<T, N> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe {
                // # Safety
                // Value previously added since len indicates so
                self.buffer[i].assume_init_drop()
            };
        }
    }
}

impl<T, const N: usize> Deref for SmallVec<T, N> {
    type Target = SmallVecBuf<T>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // SAFETY: SmallVec and SmallVecBuf have the same layout
            &*(ptr::from_raw_parts(self as *const _, N))
        }
    }
}

impl<T, const N: usize> DerefMut for SmallVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            // SAFETY: SmallVec and SmallVecBuf have the same layout
            &mut *(ptr::from_raw_parts_mut(self as *mut _, N))
        }
    }
}

impl<T> Deref for SmallVecBuf<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe {
            // # Safety
            // Values previously added since len indicates so
            self.buffer[..self.len].assume_init_ref()
        }
    }
}

impl<T> DerefMut for SmallVecBuf<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            // # Safety
            // Values previously added since len indicates so
            self.buffer[..self.len].assume_init_mut()
        }
    }
}

// SmallVec's Hash, Eq, and Ord are aliases of SmallVecBuf's
impl<T, const N: usize> Borrow<SmallVecBuf<T>> for SmallVec<T, N> {
    fn borrow(&self) -> &SmallVecBuf<T> {
        self
    }
}

// SmallVec's Hash, Eq, and Ord are aliases of SmallVecBuf's
impl<T, const N: usize> BorrowMut<SmallVecBuf<T>> for SmallVec<T, N> {
    fn borrow_mut(&mut self) -> &mut SmallVecBuf<T> {
        self
    }
}

impl<T, const N: usize, I: SliceIndex<[T]>> Index<I> for SmallVec<T, N> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        // Least ugly rust
        &(**self)[index]
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for SmallVecBuf<T> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T, const N: usize, I: SliceIndex<[T]>> IndexMut<I> for SmallVec<T, N> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut (**self)[index]
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for SmallVecBuf<T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<T: Hash> Hash for SmallVecBuf<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self[..self.len].hash(state);
    }
}

impl<T: PartialEq> PartialEq for SmallVecBuf<T> {
    fn eq(&self, other: &Self) -> bool {
        // I believe rust will by default check length first
        unsafe {
            // SAFETY: len says this is safe
            self.buffer[..self.len].assume_init_ref() == other.buffer[..other.len].assume_init_ref()
        }
    }
}

impl<T: Eq> Eq for SmallVecBuf<T> {}

impl<T: PartialOrd> PartialOrd for SmallVecBuf<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        unsafe {
            // SAFETY: len says this is safe
            self.buffer[..self.len]
                .assume_init_ref()
                .partial_cmp(other.buffer[..other.len].assume_init_ref())
        }
    }
}

impl<T: Ord> Ord for SmallVecBuf<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        unsafe {
            // SAFETY: len says this is safe
            self.buffer[..self.len]
                .assume_init_ref()
                .cmp(other.buffer[..other.len].assume_init_ref())
        }
    }
}

impl<T: Hash, const N: usize> Hash for SmallVec<T, N> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: PartialEq, const N: usize> PartialEq for SmallVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        (**self).eq(other)
    }
}

impl<T: Eq, const N: usize> Eq for SmallVec<T, N> {}

impl<T: PartialOrd, const N: usize> PartialOrd for SmallVec<T, N> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<T: Ord, const N: usize> Ord for SmallVec<T, N> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (**self).cmp(other)
    }
}

impl<T: Debug, const N: usize> Debug for SmallVec<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T, const N: usize> Default for SmallVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// # Safety
/// buffer[..*len] is assumed to be initialized
#[inline(always)]
unsafe fn common_push<'a, T>(
    buffer: &'a mut [MaybeUninit<T>],
    len: &'a mut usize,
    cap: usize,
    value: T,
) -> Result<&'a T, T> {
    if *len == cap {
        return Err(value);
    }
    buffer[*len] = MaybeUninit::new(value);
    let r = unsafe {
        // # Safety
        // Just made the sucker
        buffer[*len].assume_init_ref()
    };
    *len += 1;
    Ok(r)
}

/// # Safety
/// buffer[..*len] is assumed to be initialized
#[inline(always)]
unsafe fn common_pop<T>(buffer: &[MaybeUninit<T>], len: &mut usize) -> Option<T> {
    if *len == 0 {
        return None;
    }

    *len -= 1;
    Some(unsafe {
        // # Safety
        // Value previously added since len indicates so
        // Move is fine since we promise not to give it again unless added back as next push
        buffer[*len].assume_init_read()
    })
}

/// # Safety
/// buffer[..*len] is assumed to be initialized
#[inline(always)]
unsafe fn common_erase<T>(
    buffer: &mut [MaybeUninit<T>],
    len: &mut usize,
    index: usize,
) -> Option<T> {
    if *len <= index {
        return None;
    }
    buffer[index..*len].rotate_left(1);
    unsafe { common_pop(buffer, len) }
}

/// # Safety
/// buffer[..*len] is assumed to be initialized
#[inline(always)]
unsafe fn common_erase_value<T: PartialEq, U: Borrow<T>>(
    buffer: &mut [MaybeUninit<T>],
    len: &mut usize,
    val: U,
) -> Option<T> {
    let ibuffer = unsafe {
        // SAFETY: guarenteed by caller
        buffer[..*len].assume_init_mut()
    };
    let index = ibuffer.iter().position(|v| v == val.borrow())?;
    unsafe { common_erase(buffer, len, index) }
}
