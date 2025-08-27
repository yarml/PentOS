use x64::mem::{
    MemorySize,
    frame::size::{Frame2MiB, Frame4KiB, Frame8MiB, Frame64KiB, Frame128KiB, FrameSize},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MidPageSize {
    K4,
    K64,
    K128,
    M2,
    M8,
}

impl MidPageSize {
    pub fn from_size(size: usize) -> Option<Self> {
        match size {
            Frame4KiB::SIZE => Some(Self::K4),
            Frame64KiB::SIZE => Some(Self::K64),
            Frame128KiB::SIZE => Some(Self::K128),
            Frame2MiB::SIZE => Some(Self::M2),
            Frame8MiB::SIZE => Some(Self::M8),
            _ => None,
        }
    }
}

impl MidPageSize {
    const ORDERS: [usize; 5] = [0, 4, 5, 9, 11];

    pub const fn next_size(&self) -> Option<Self> {
        match self {
            Self::K4 => Some(Self::K64),
            Self::K64 => Some(Self::K128),
            Self::K128 => Some(Self::M2),
            Self::M2 => Some(Self::M2),
            Self::M8 => None,
        }
    }

    pub const fn prev_size(&self) -> Option<Self> {
        match self {
            Self::K4 => None,
            Self::K64 => Some(Self::K4),
            Self::K128 => Some(Self::K64),
            Self::M2 => Some(Self::K128),
            Self::M8 => Some(Self::M2),
        }
    }

    pub const fn size(&self) -> MemorySize {
        match self {
            Self::K4 => MemorySize::new(4 * 1024),
            Self::K64 => MemorySize::new(64 * 1024),
            Self::K128 => MemorySize::new(128 * 1024),
            Self::M2 => MemorySize::new(2 * 1024 * 1024),
            Self::M8 => MemorySize::new(8 * 1024 * 1024),
        }
    }

    #[inline]
    pub const fn order(&self) -> usize {
        Self::ORDERS[self.index()]
    }

    pub const fn index(&self) -> usize {
        match self {
            Self::K4 => 0,
            Self::K64 => 1,
            Self::K128 => 2,
            Self::M2 => 3,
            Self::M8 => 4,
        }
    }
}
