use core::marker::PhantomData;

#[derive(Clone, Copy)]
pub struct Frame4KiB;

#[derive(Clone, Copy)]
pub struct Frame2MiB;

#[derive(Clone, Copy)]
pub struct Frame1GiB;

#[derive(Clone, Copy)]
pub struct Frame512GiB;

#[derive(Clone, Copy)]
pub struct FrameDynSize<const SIZE: usize>;

#[derive(Clone, Copy)]
pub struct FrameInvalidSize;

pub type FrameMaxSize = Frame512GiB;

pub trait FrameSize: Clone + Copy {
    const SHIFT: usize;
    const SIZE: usize = 1 << Self::SHIFT;
    const MASK: usize = usize::MAX >> Self::SHIFT << Self::SHIFT;
    const ORDER: usize = Self::SHIFT - 12;
}

impl FrameSize for Frame4KiB {
    const SHIFT: usize = 12;
}

impl FrameSize for Frame2MiB {
    const SHIFT: usize = 21;
}

impl FrameSize for Frame1GiB {
    const SHIFT: usize = 30;
}

impl FrameSize for Frame512GiB {
    const SHIFT: usize = 39;
}

impl<const SIZE: usize> FrameSize for FrameDynSize<SIZE> {
    const SHIFT: usize = if SIZE.is_power_of_two() {
        SIZE.trailing_zeros() as usize
    } else {
        panic!("Dynamic frame size must be a power of 2");
    };
}
pub struct FrameSizeOps<FS1: FrameSize, FS2: FrameSize> {
    phantom: PhantomData<(FS1, FS2)>,
}

impl<FS1: FrameSize, FS2: FrameSize> FrameSizeOps<FS1, FS2> {
    pub const ORDER_DIFF: usize = FS1::ORDER - FS2::ORDER;
    pub const FRAME_COUNT_DIFF: usize = 2usize.pow(Self::ORDER_DIFF as u32);
}
