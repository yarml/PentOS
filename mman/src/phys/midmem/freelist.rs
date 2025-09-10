use {
    super::{BLOCK_SIZE, size::MidFrameSize},
    common::collections::smallvec::{SmallVec, SmallVecBuf},
    x64::mem::frame::{Frame, FrameRange, size::Frame4KiB},
};

macro_rules! freelist_decl {
    ($size:ident) => {
        SmallVec<u32, { BLOCK_SIZE / MidFrameSize::$size.size() }>
    };
}

/// Per block freelist. Has a constant size allocated at kernel load time (bss section).
/// Manages the freelists for pges of sizes: 4K, 64K, 128K, 2M within the block it is contained within.
pub struct Freelist {
    k4: freelist_decl!(K4),
    k64: freelist_decl!(K64),
    k128: freelist_decl!(K128),
    m2: freelist_decl!(M2),
    m8: freelist_decl!(M8),
}

impl Freelist {
    pub fn pop(&mut self, size: MidFrameSize) -> Option<FrameRange<Frame4KiB>> {
        let list = self.getlist(size);
        list.pop()
            .map(|index| FrameRange::new(Frame::from_number(index as usize), size.k4_count()))
    }

    pub fn push(&mut self, frame: FrameRange<Frame4KiB>) {
        let size = MidFrameSize::from_size(*frame.size());
        assert!(*frame.start().boundary() % size.alignment() == 0);
        let list = self.getlist(size);
        list.push(*frame.start().boundary() as u32).unwrap();
    }
}

impl Freelist {
    fn getlist(&mut self, size: MidFrameSize) -> &mut SmallVecBuf<u32> {
        match size {
            MidFrameSize::K4 => &mut self.k4,
            MidFrameSize::K64 => &mut self.k64,
            MidFrameSize::K128 => &mut self.k128,
            MidFrameSize::M2 => &mut self.m2,
            MidFrameSize::M8 => &mut self.m8,
        }
    }
}
