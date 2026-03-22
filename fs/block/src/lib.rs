#![no_std]
extern crate alloc;

use {alloc::boxed::Box, core::pin::Pin, io::IoResult};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockDeviceSize {
    /// Logical block size in bytes. Must be a power of 2 & >= 512.
    pub sector_size: usize,
    /// Total number of addressable sectors on this device.
    pub sector_count: u64,
}

pub trait BlockDevice {
    fn size(&self) -> BlockDeviceSize;

    /// Read sectors at `lba` into `buf`.
    /// `buf.len()` must be a multiple of `self.sector_size()`.
    fn read_sectors<'a>(
        &'a mut self,
        lba: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;

    /// Write sectors at `lba` from `buf`.
    /// `buf.len()` must be a multiple of `self.sector_size()`.
    fn write_sectors<'a>(
        &'a mut self,
        lba: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;
}
