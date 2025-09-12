use x64::mem::frame::size::{Frame2MiB, Frame4KiB, FrameDynSize, FrameSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MidFrameSize {
    K4,
    K64,
    K128,
    M2,
    M8,
}

pub struct MidFrameSizeIterator {
    next: Option<MidFrameSize>,
}

impl MidFrameSize {
    pub fn from_size(size: usize) -> Self {
        match size {
            Frame4KiB::SIZE => MidFrameSize::K4,
            FrameDynSize::<{ 64 * 1024 }>::SIZE => MidFrameSize::K64,
            FrameDynSize::<{ 128 * 1024 }>::SIZE => MidFrameSize::K128,
            Frame2MiB::SIZE => MidFrameSize::M2,
            FrameDynSize::<{ 8 * 1024 * 1024 }>::SIZE => MidFrameSize::M8,
            _ => unreachable!(""),
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
    pub const fn children_count(&self) -> Option<usize> {
        // Somehow ? operator cannot be used in const functions, but if let can
        let child = if let Some(child_order) = self.child_order() {
            child_order
        } else {
            return None;
        };
        let order_diff = self.order() - child.order();
        Some(1 << order_diff)
    }
    pub const fn children_mask(&self) -> Option<u64> {
        let children_count = if let Some(children_count) = self.children_count() {
            children_count
        } else {
            return None;
        };
        Some(u64::MAX >> (64 - children_count))
    }
    pub const fn k4_count(&self) -> usize {
        1 << self.order()
    }
    pub const fn size(&self) -> usize {
        self.k4_count() * Frame4KiB::SIZE
    }
    pub const fn alignment(&self) -> usize {
        self.size()
    }

    pub const fn child_order(&self) -> Option<Self> {
        match self {
            MidFrameSize::K4 => None,
            MidFrameSize::K64 => Some(MidFrameSize::K4),
            MidFrameSize::K128 => Some(MidFrameSize::K64),
            MidFrameSize::M2 => Some(MidFrameSize::K128),
            MidFrameSize::M8 => Some(MidFrameSize::M2),
        }
    }
    pub const fn parent_order(&self) -> Option<Self> {
        match self {
            MidFrameSize::K4 => Some(MidFrameSize::K64),
            MidFrameSize::K64 => Some(MidFrameSize::K128),
            MidFrameSize::K128 => Some(MidFrameSize::M2),
            MidFrameSize::M2 => Some(MidFrameSize::M8),
            MidFrameSize::M8 => None,
        }
    }
}

impl IntoIterator for MidFrameSize {
    type Item = MidFrameSize;
    type IntoIter = MidFrameSizeIterator;

    fn into_iter(self) -> Self::IntoIter {
        MidFrameSizeIterator { next: Some(self) }
    }
}

impl Iterator for MidFrameSizeIterator {
    type Item = MidFrameSize;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = current.child_order();
        Some(current)
    }
}

impl DoubleEndedIterator for MidFrameSizeIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        let current = self.next?;
        self.next = current.parent_order();
        Some(current)
    }
}
