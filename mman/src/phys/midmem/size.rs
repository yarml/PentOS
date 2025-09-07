use x64::mem::frame::size::{Frame4KiB, FrameSize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MidFrameSize {
    K4,
    K64,
    K128,
    M2,
    M8,
}

impl MidFrameSize {
    pub const fn parent_level(&self) -> Option<Self> {
        match self {
            MidFrameSize::K4 => Some(Self::K64),
            MidFrameSize::K64 => Some(Self::K128),
            MidFrameSize::K128 => Some(Self::M2),
            MidFrameSize::M2 => Some(Self::M8),
            MidFrameSize::M8 => None,
        }
    }
    pub const fn order(&self) -> usize {
        match self {
            MidFrameSize::K4 => 0,
            MidFrameSize::K64 => 4,
            MidFrameSize::K128 => 5,
            MidFrameSize::M2 => 9,
            MidFrameSize::M8 => 11,
        }
    }
    pub const fn is_top_level(&self) -> bool {
        self.parent_level().is_none()
    }
    pub const fn k4_count(&self) -> usize {
        1 << self.order()
    }
    pub const fn size(&self) -> usize {
        self.k4_count() * Frame4KiB::SIZE
    }
}
