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
    pub const fn parent_level(&self) -> Option<Self> {
        match self {
            LowMemFrameSize::K64 => Some(Self::K128),
            LowMemFrameSize::K128 => None,
        }
    }
    pub const fn order(&self) -> usize {
        match self {
            LowMemFrameSize::K64 => 4,
            LowMemFrameSize::K128 => 5,
        }
    }

    pub const fn is_top_level(&self) -> bool {
        self.parent_level().is_none()
    }
    pub const fn k4_count(&self) -> usize {
        2usize.pow(self.order() as u32)
    }
    pub const fn size(&self) -> usize {
        self.k4_count() * Frame4KiB::SIZE
    }
}
