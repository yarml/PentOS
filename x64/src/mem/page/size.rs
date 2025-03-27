use crate::mem::frame::size::Frame1GiB;
use crate::mem::frame::size::Frame2MiB;
use crate::mem::frame::size::Frame4KiB;
use crate::mem::frame::size::FrameSize;

#[derive(Clone, Copy)]
pub struct Page4KiB;
#[derive(Clone, Copy)]
pub struct Page2MiB;
#[derive(Clone, Copy)]
pub struct Page1GiB;

#[derive(Clone, Copy)]
pub struct PageInvalidSize;

pub trait PageSize {
    type PhysicalPageSize: FrameSize;

    const SHIFT: usize = Self::PhysicalPageSize::SHIFT;
    const SIZE: usize = Self::PhysicalPageSize::SIZE;
    const MASK: usize = Self::PhysicalPageSize::MASK;

    // Paging structure related
    const PAT_INDEX: usize;
    const USE_MAP_FLAG: u64;

    const PAT_MASK: u64 = 0b1 << Self::PAT_INDEX | (0b11 << 2);

    type NextPageSize;
}

impl PageSize for Page4KiB {
    type PhysicalPageSize = Frame4KiB;

    const PAT_INDEX: usize = 7;
    const USE_MAP_FLAG: u64 = 0;

    type NextPageSize = PageInvalidSize;
}

impl PageSize for Page2MiB {
    type PhysicalPageSize = Frame2MiB;

    const PAT_INDEX: usize = 12;
    const USE_MAP_FLAG: u64 = 1 << 7;

    type NextPageSize = Page4KiB;
}

impl PageSize for Page1GiB {
    type PhysicalPageSize = Frame1GiB;

    const PAT_INDEX: usize = 12;
    const USE_MAP_FLAG: u64 = 1 << 7;

    type NextPageSize = Page2MiB;
}
