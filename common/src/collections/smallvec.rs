use core::{
    fmt::Debug,
    hash::Hash,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, DerefPure, Index, IndexMut},
    slice::{Iter, IterMut, SliceIndex},
};

/// A vector that contains all its element with its allocation unit
/// The size of the SmallVec depends on its capacity which cannot be changed
/// and has to be known at compile time
#[repr(C)]
pub struct SmallVec<T, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    len: usize,
}

/// A mutable access to a SmallVec with runtime capacity tracking. Needs a SmallVec to contain the data.
pub struct SmallVecMut<'a, T> {
    buffer: &'a mut [MaybeUninit<T>],
    len: &'a mut usize,
    capacity: usize,
}

impl<T, const N: usize> SmallVec<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }
}

impl<T, const N: usize> SmallVec<T, N> {
    pub const CAPACITY: usize = N;

    #[must_use = "check that value was added, otherwise it will just drop"]
    pub fn push(&mut self, value: T) -> Result<&T, T> {
        common_push(&mut self.buffer, &mut self.len, N, value)
    }
    pub fn pop(&mut self) -> Option<T> {
        common_pop(&self.buffer, &mut self.len)
    }

    pub fn erase(&mut self, index: usize) -> Option<T> {
        common_erase(&mut self.buffer, &mut self.len, index)
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn capacity(&self) -> usize {
        Self::CAPACITY
    }

    pub fn as_mut(&mut self) -> SmallVecMut<T> {
        SmallVecMut {
            buffer: &mut self.buffer,
            len: &mut self.len,
            capacity: N,
        }
    }
}

impl<'a, T> SmallVecMut<'a, T> {
    #[must_use = "check that value was added, otherwise it will just drop"]
    pub fn push(&mut self, value: T) -> Result<&T, T> {
        common_push(self.buffer, self.len, self.capacity, value)
    }
    pub fn pop(&mut self) -> Option<T> {
        common_pop(self.buffer, self.len)
    }
    pub fn erase(&mut self, index: usize) -> Option<T> {
        common_erase(self.buffer, self.len, index)
    }

    pub fn len(&self) -> usize {
        *self.len
    }
    pub fn is_empty(&self) -> bool {
        *self.len == 0
    }
    pub fn capacity(&self) -> usize {
        self.capacity
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
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe {
            // # Safety
            // Values previously added since len indicates so
            self.buffer[..self.len].assume_init_ref()
        }
    }
}

impl<T, const N: usize> DerefMut for SmallVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            // # Safety
            // Values previously added since len indicates so
            self.buffer[..self.len].assume_init_mut()
        }
    }
}

/// # Safety
/// Since DerefPure is unstable this needs to be checked whenever the compiler is updated
/// For now, consecutive calls to Deref and DerefMut will always give the same value if no mutation
/// happens in between
unsafe impl<T, const N: usize> DerefPure for SmallVec<T, N> {}

impl<T, const N: usize, I: SliceIndex<[T]>> Index<I> for SmallVec<T, N> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T, const N: usize, I: SliceIndex<[T]>> IndexMut<I> for SmallVec<T, N> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<T: Hash, const N: usize> Hash for SmallVec<T, N> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self[..self.len].hash(state);
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

fn common_push<'a, T>(
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

fn common_pop<T>(buffer: &[MaybeUninit<T>], len: &mut usize) -> Option<T> {
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

pub fn common_erase<T>(buffer: &mut [MaybeUninit<T>], len: &mut usize, index: usize) -> Option<T> {
    if *len <= index {
        return None;
    }
    buffer[index..*len].rotate_left(1);
    common_pop(buffer, len)
}
