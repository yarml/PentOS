pub mod size;

use {
    super::{
        addr::Address,
        page::{Page, size::PageSize},
    },
    crate::mem::{MemorySize, addr::PhysAddr, frame::size::Frame4KiB},
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
pub struct FrameIter<S: FrameSize> {
    start: Frame<S>,
    current: usize,
    count: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FrameRangeIter<S: FrameSize> {
    start: FrameRange<S>,
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

    #[inline(always)]
    pub const fn size(&self) -> MemorySize {
        MemorySize::new(S::SIZE)
    }

    #[inline(always)]
    pub const fn order(&self) -> usize {
        S::ORDER
    }
}

impl<S: FrameSize> Frame<S> {
    #[inline(always)]
    pub fn to_virt<VS: PageSize>(&self) -> Page<VS> {
        assert!(VS::SIZE == S::SIZE);
        Page::containing(self.boundary.to_virt())
    }

    pub const fn into_range<SmallerSize: FrameSize>(&self) -> FrameRange<SmallerSize> {
        assert!(SmallerSize::SIZE < S::SIZE);
        let count = S::SIZE / SmallerSize::SIZE;
        FrameRange {
            start: Frame::containing(self.boundary),
            count,
        }
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
        assert!((start.size().as_usize() * N).is_power_of_two());
        Self { start }
    }
}
impl<S: FrameSize> FrameRange<S> {
    pub const fn new(start: Frame<S>, count: usize) -> Self {
        assert!((start.size().as_usize() * count).is_power_of_two());
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
    pub const fn size(&self) -> MemorySize {
        MemorySize::new(self.start.size().as_usize() * self.count())
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
    pub const fn size(&self) -> MemorySize {
        MemorySize::new(self.start.size().as_usize() * self.count())
    }
    pub const fn order(&self) -> usize {
        (self.size().as_usize() / Frame4KiB::SIZE).trailing_zeros() as usize
    }
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub const fn into_extent<const N: usize>(&self) -> FrameExtent<S, N> {
        assert!(self.count == N);
        FrameExtent { start: self.start }
    }
    pub const fn into_frame<LargerSize: FrameSize>(&self) -> Frame<LargerSize> {
        assert!(
            self.start.boundary.as_usize()
                == Frame::<LargerSize>::containing(self.start.boundary)
                    .boundary
                    .as_usize()
        );
        Frame::containing(self.start.boundary)
    }

    /// Splits the frame range to frame ranges of order `order`
    pub const fn split<SmallerSize: FrameSize>(&self, order: usize) -> FrameRangeIter<SmallerSize> {
        let size_per_range = (1 << order) * Frame4KiB::SIZE;
        assert!(size_per_range.is_multiple_of(SmallerSize::SIZE));
        let total_frame_count = self.size().as_usize() / SmallerSize::SIZE;
        let frame_count_per_range = size_per_range / SmallerSize::SIZE;
        FrameRangeIter::<SmallerSize> {
            start: FrameRange::new(
                Frame::containing(self.start.boundary),
                frame_count_per_range,
            ),
            count: total_frame_count,
            current: 0,
        }
    }
}

impl<S: FrameSize, const N: usize> IntoIterator for FrameExtent<S, N> {
    type Item = Frame<S>;
    type IntoIter = FrameIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        FrameIter {
            start: self.start,
            current: Self::COUNT,
            count: 0,
        }
    }
}
impl<S: FrameSize> IntoIterator for FrameRange<S> {
    type Item = Frame<S>;
    type IntoIter = FrameIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        FrameIter {
            start: self.start,
            count: self.count,
            current: 0,
        }
    }
}

impl<S: FrameSize> Iterator for FrameIter<S> {
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

impl<S: FrameSize> Iterator for FrameRangeIter<S> {
    type Item = FrameRange<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.count {
            return None;
        }
        let frame = FrameRange::new(
            self.start.start + self.current * self.start.count,
            self.start.count,
        );
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

impl<S: FrameSize, const N: usize> Debug for FrameExtent<S, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Frames{}#{}@{}:{}",
            MemorySize::new(S::SIZE),
            self.start().number(),
            self.start().boundary(),
            self.count()
        )
    }
}
impl<S: FrameSize> Debug for FrameRange<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Frames{}#{}@{}:{}",
            MemorySize::new(S::SIZE),
            self.start().number(),
            self.start().boundary(),
            self.count()
        )
    }
}

impl<S: FrameSize> Display for Frame<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Frame{}#{}", MemorySize::new(S::SIZE), self.number())
    }
}
impl<S: FrameSize, const N: usize> Display for FrameExtent<S, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Frame{}#{}:{}",
            MemorySize::new(S::SIZE),
            self.start().number(),
            self.count()
        )
    }
}
impl<S: FrameSize> Display for FrameRange<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Frame{}#{}:{}",
            MemorySize::new(S::SIZE),
            self.start().number(),
            self.count()
        )
    }
}
