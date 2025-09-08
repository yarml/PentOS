use x64::mem::frame::{
    Frame,
    size::{Frame4KiB, FrameDynSize, FrameSize},
};

pub type LowMemFrame64KiB = Frame<FrameDynSize<{ 64 * 1024 }>>;
pub type LowMemFrame128KiB = Frame<FrameDynSize<{ 64 * 1024 }>>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LowMemFrameSize {
    K64,
    K128,
}

impl LowMemFrameSize {
    pub const fn order(&self) -> usize {
        match self {
            LowMemFrameSize::K64 => 4,
            LowMemFrameSize::K128 => 5,
        }
    }

    pub const fn k4_count(&self) -> usize {
        1 << self.order()
    }
    pub const fn size(&self) -> usize {
        self.k4_count() * Frame4KiB::SIZE
    }
}
