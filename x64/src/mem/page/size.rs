use crate::mem::MemorySize;
use crate::mem::frame::size::Frame1GiB;
use crate::mem::frame::size::Frame2MiB;
use crate::mem::frame::size::Frame4KiB;
use crate::mem::frame::size::FrameInvalidSize;
use crate::mem::frame::size::FrameSize;

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

pub trait PageSize: Clone + Copy {
    type PhysicalPageSize;

    const SHIFT: usize;
    const SIZE: MemorySize;
    const MASK: usize;

    // Paging structure related
    const PAT_INDEX: usize;
    const USE_MAP_FLAG: u64;

    const PAT_MASK: u64 = 0b1 << Self::PAT_INDEX | (0b11 << 2);

    type ReferenceTarget;
}

impl PageSize for Page4KiB {
    type PhysicalPageSize = Frame4KiB;

    const SHIFT: usize = Self::PhysicalPageSize::SHIFT;
    const SIZE: MemorySize = Self::PhysicalPageSize::SIZE;
    const MASK: usize = Self::PhysicalPageSize::MASK;

    const PAT_INDEX: usize = 7;
    const USE_MAP_FLAG: u64 = 0;

    type ReferenceTarget = PageInvalidSize;
}

impl PageSize for Page2MiB {
    type PhysicalPageSize = Frame2MiB;

    const SHIFT: usize = Self::PhysicalPageSize::SHIFT;
    const SIZE: MemorySize = Self::PhysicalPageSize::SIZE;
    const MASK: usize = Self::PhysicalPageSize::MASK;

    const PAT_INDEX: usize = 12;
    const USE_MAP_FLAG: u64 = 1 << 7;

    type ReferenceTarget = Page4KiB;
}

impl PageSize for Page1GiB {
    type PhysicalPageSize = Frame1GiB;

    const SHIFT: usize = Self::PhysicalPageSize::SHIFT;
    const SIZE: MemorySize = Self::PhysicalPageSize::SIZE;
    const MASK: usize = Self::PhysicalPageSize::MASK;

    const PAT_INDEX: usize = 12;
    const USE_MAP_FLAG: u64 = 1 << 7;

    type ReferenceTarget = Page2MiB;
}

impl PageSize for Page512GiB {
    type PhysicalPageSize = FrameInvalidSize;

    const SHIFT: usize = 39;
    const SIZE: MemorySize = MemorySize::new(1 << Self::SHIFT);
    const MASK: usize = usize::MAX >> Self::SHIFT << Self::SHIFT;

    const PAT_INDEX: usize = 0;
    const USE_MAP_FLAG: u64 = 0;

    type ReferenceTarget = Page1GiB;
}
