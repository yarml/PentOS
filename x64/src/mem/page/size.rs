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

pub trait PageSize {
    type PhysicalPageSize: FrameSize;

    const SHIFT: usize = Self::PhysicalPageSize::SHIFT;
    const SIZE: usize = Self::PhysicalPageSize::SIZE;
    const MASK: usize = Self::PhysicalPageSize::MASK;
}

impl PageSize for Page4KiB {
    type PhysicalPageSize = Frame4KiB;
}

impl PageSize for Page2MiB {
    type PhysicalPageSize = Frame2MiB;
}

impl PageSize for Page1GiB {
    type PhysicalPageSize = Frame1GiB;
}
