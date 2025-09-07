pub mod size;

use {
    super::{
        addr::Address,
        page::{Page, size::PageSize},
    },
    crate::mem::{MemorySize, addr::PhysAddr},
    core::{
        fmt::{Debug, Display},
        marker::PhantomData,
        ops::Add,
    },
    size::FrameSize,
};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame<S: FrameSize> {
    boundary: PhysAddr,
    _phantom: PhantomData<S>,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameExtent<S: FrameSize, const N: usize> {
    start: Frame<S>,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FrameRange<S: FrameSize> {
    start: Frame<S>,
    count: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FrameRangeIter<S: FrameSize> {
    start: Frame<S>,
    current: usize,
    count: usize,
}

impl<S: FrameSize> Frame<S> {
    #[inline(always)]
    pub const fn containing(addr: PhysAddr) -> Self {
        Self {
            boundary: PhysAddr::new_panic(addr.as_usize() & S::MASK),
            _phantom: PhantomData,
        }
    }
    #[inline(always)]
    pub const fn from_number(num: usize) -> Self {
        Self {
            boundary: PhysAddr::new_panic(num << S::SHIFT),
            _phantom: PhantomData,
        }
    }
}

impl<S: FrameSize> Frame<S> {
    #[inline(always)]
    pub const fn boundary(&self) -> PhysAddr {
        self.boundary
    }

    #[inline(always)]
    pub const fn number(&self) -> usize {
        self.boundary.as_usize() >> S::SHIFT
    }
}

impl<S: FrameSize> Frame<S> {
    #[inline(always)]
    pub fn to_virt<VS: PageSize>(&self) -> Page<VS> {
        assert!(VS::SIZE == S::SIZE);
        Page::containing(self.boundary.to_virt())
    }
}

impl<S: FrameSize> Add<usize> for Frame<S> {
    type Output = Frame<S>;

    fn add(self, rhs: usize) -> Self::Output {
        Self::from_number(self.number() + rhs)
    }
}

impl<S: FrameSize, const N: usize> FrameExtent<S, N> {
    pub const COUNT: usize = N;
    pub const IS_EMPTY: bool = Self::COUNT == 0;
    pub const fn new(start: Frame<S>) -> Self {
        Self { start }
    }
}
impl<S: FrameSize> FrameRange<S> {
    pub const fn new(start: Frame<S>, count: usize) -> Self {
        Self { start, count }
    }
}

impl<S: FrameSize, const N: usize> FrameExtent<S, N> {
    pub const fn start(&self) -> Frame<S> {
        self.start
    }
    pub const fn count(&self) -> usize {
        Self::COUNT
    }
    pub const fn is_empty(&self) -> bool {
        Self::COUNT == 0
    }

    pub const fn into_range(&self) -> FrameRange<S> {
        FrameRange {
            start: self.start,
            count: Self::COUNT,
        }
    }
}
impl<S: FrameSize> FrameRange<S> {
    pub const fn start(&self) -> Frame<S> {
        self.start
    }

    pub const fn count(&self) -> usize {
        self.count
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub const fn into_extent<const N: usize>(&self) -> FrameExtent<S, N> {
        assert!(self.count == N);
        FrameExtent { start: self.start }
    }
}

impl<S: FrameSize, const N: usize> IntoIterator for FrameExtent<S, N> {
    type Item = Frame<S>;
    type IntoIter = FrameRangeIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        FrameRangeIter {
            start: self.start,
            current: Self::COUNT,
            count: 0,
        }
    }
}
impl<S: FrameSize> IntoIterator for FrameRange<S> {
    type Item = Frame<S>;
    type IntoIter = FrameRangeIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        FrameRangeIter {
            start: self.start,
            count: self.count,
            current: 0,
        }
    }
}

impl<S: FrameSize> Iterator for FrameRangeIter<S> {
    type Item = Frame<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.count {
            return None;
        }
        let frame = self.start + self.current;
        self.current += 1;
        Some(frame)
    }
}

impl<S: FrameSize> Debug for Frame<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Frame{}#{}@{}",
            MemorySize::new(S::SIZE),
            self.number(),
            self.boundary()
        )
    }
}

impl<S: FrameSize> Display for Frame<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Frame{}#{}", MemorySize::new(S::SIZE), self.number())
    }
}
