use {
    crate::kalloc::midmem::slab::SlabHeader,
    core::mem,
    x64::mem::frame::size::{Frame4KiB, FrameSize},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(usize)]
pub enum BucketSize {
    B8 = 8,
    B16 = 16,
    B32 = 32,
    B64 = 64,
    B128 = 128,
    B256 = 256,
    B512 = 512,
    B1K = 1024,
}

impl BucketSize {
    pub const COUNT: usize = 8;
    pub const MAX_SLAB_ALLOC: usize = Self::B1K as usize;

    pub const ALL: [Self; Self::COUNT] = [
        Self::B8,
        Self::B16,
        Self::B32,
        Self::B64,
        Self::B128,
        Self::B256,
        Self::B512,
        Self::B1K,
    ];

    #[inline]
    pub fn fit(size: usize) -> Option<Self> {
        Self::ALL.iter().copied().find(|&b| b as usize >= size)
    }

    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Self::B8 => 0,
            Self::B16 => 1,
            Self::B32 => 2,
            Self::B64 => 3,
            Self::B128 => 4,
            Self::B256 => 5,
            Self::B512 => 6,
            Self::B1K => 7,
        }
    }

    #[inline]
    pub const fn slots_per_slab(self) -> usize {
        let header_size = mem::size_of::<SlabHeader>();
        let slot_size = self as usize;

        let slots_start = header_size.next_multiple_of(slot_size);
        let slots_end = Frame4KiB::SIZE;

        if slots_end <= slots_start {
            0
        } else {
            (slots_end - slots_start) / slot_size
        }
    }
}
