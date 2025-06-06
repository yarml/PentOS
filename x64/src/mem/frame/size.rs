#[derive(Clone, Copy)]
pub struct Frame4KiB;
#[derive(Clone, Copy)]
pub struct Frame64KiB;
#[derive(Clone, Copy)]
pub struct Frame128KiB;
#[derive(Clone, Copy)]
pub struct Frame2MiB;
#[derive(Clone, Copy)]
pub struct Frame1GiB;
#[derive(Clone, Copy)]
pub struct FrameInvalidSize;

pub trait FrameSize: Clone + Copy {
    const SHIFT: usize;
    const SIZE: usize = 1 << Self::SHIFT;
    const MASK: usize = usize::MAX >> Self::SHIFT << Self::SHIFT;
}

impl FrameSize for Frame4KiB {
    const SHIFT: usize = 12;
}

impl FrameSize for Frame64KiB {
    const SHIFT: usize = 16;
}

impl FrameSize for Frame128KiB {
    const SHIFT: usize = 17;
}

impl FrameSize for Frame2MiB {
    const SHIFT: usize = 21;
}

impl FrameSize for Frame1GiB {
    const SHIFT: usize = 30;
}
