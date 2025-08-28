use {
    super::{MIDMEM_SIZE, size::MidPageSize},
    common::collections::smallvec::{SmallVec, SmallVecBuf},
    x64::mem::frame::{
        Frame,
        size::{Frame4KiB, FrameSize},
    },
};

macro_rules! freelist_decl {
    ($size:ident) => {
        SmallVec<u32, { MIDMEM_SIZE / MidPageSize::$size.size().as_usize() }>
    };
}

/// Middle memory freelist. Has a constant size allocated at kernel load time (data section).
/// Manages the freelists for pages of sizes: 4K, 64K, 128K, 2M, 8M
/// This concerns middle memory only [16M-4G)
pub struct Freelist {
    k4: freelist_decl!(K4),
    k64: freelist_decl!(K64),
    k128: freelist_decl!(K128),
    m2: freelist_decl!(M2),
    m8: freelist_decl!(M8),
}

impl Freelist {
    pub fn alloc<S: FrameSize>(&mut self) -> Option<Frame<Frame4KiB>> {
        let mut list = self.getvec(MidPageSize::from_size(S::SIZE).unwrap());
        list.pop().map(|index| Frame::from_number(index as usize))
    }
}

impl Freelist {
    fn getvec(&mut self, size: MidPageSize) -> &mut SmallVecBuf<u32> {
        match size {
            MidPageSize::K4 => &mut self.k4,
            MidPageSize::K64 => &mut self.k64,
            MidPageSize::K128 => &mut self.k128,
            MidPageSize::M2 => &mut self.m2,
            MidPageSize::M8 => &mut self.m8,
        }
    }
}
