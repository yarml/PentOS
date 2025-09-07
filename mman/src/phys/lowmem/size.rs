use x64::mem::frame::{
    FrameExtent,
    size::{Frame4KiB, FrameSize},
};

pub type LowMemFrame64KiB = FrameExtent<Frame4KiB, { LowMemFrameSize::K64.k4_count() }>;
pub type LowMemFrame128KiB = FrameExtent<Frame4KiB, { LowMemFrameSize::K128.k4_count() }>;

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
