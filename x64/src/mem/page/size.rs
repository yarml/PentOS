use crate::mem::frame::size::{Frame1GiB, Frame2MiB, Frame4KiB, Frame512GiB, FrameSize};

#[derive(Clone, Copy)]
pub struct Page4KiB;

#[derive(Clone, Copy)]
pub struct Page2MiB;

#[derive(Clone, Copy)]
pub struct Page1GiB;

#[derive(Clone, Copy)]
pub struct Page512GiB;

#[derive(Clone, Copy)]
pub struct PageInvalidSize;

// FIXME: Is it a good idea to encode PAT management related info
// within the x64 crate? Not a big deal, but probably could be done better
pub trait PageSize: Clone + Copy {
    type PhysicalPageSize: FrameSize;

    const SHIFT: usize = Self::PhysicalPageSize::SHIFT;
    const SIZE: usize = Self::PhysicalPageSize::SIZE;
    const MASK: usize = Self::PhysicalPageSize::MASK;

    // Paging structure related
    const PAT_INDEX: usize;
    const USE_MAP_FLAG: u64;

    const PAT_MASK: u64 = 0b1 << Self::PAT_INDEX | (0b11 << 2);

    type ReferenceTarget;
}

impl PageSize for Page4KiB {
    type PhysicalPageSize = Frame4KiB;

    const PAT_INDEX: usize = 7;
    const USE_MAP_FLAG: u64 = 0;

    type ReferenceTarget = PageInvalidSize;
}

impl PageSize for Page2MiB {
    type PhysicalPageSize = Frame2MiB;

    const PAT_INDEX: usize = 12;
    const USE_MAP_FLAG: u64 = 1 << 7;

    type ReferenceTarget = Page4KiB;
}

impl PageSize for Page1GiB {
    type PhysicalPageSize = Frame1GiB;

    const PAT_INDEX: usize = 12;
    const USE_MAP_FLAG: u64 = 1 << 7;

    type ReferenceTarget = Page2MiB;
}

impl PageSize for Page512GiB {
    type PhysicalPageSize = Frame512GiB;

    const PAT_INDEX: usize = 0;
    const USE_MAP_FLAG: u64 = 0;

    type ReferenceTarget = Page1GiB;
}
