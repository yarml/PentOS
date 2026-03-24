#![no_std]
extern crate alloc;

use {
    alloc::{boxed::Box, vec},
    core::pin::Pin,
    io::IoResult,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockDeviceSize {
    /// Logical block size in bytes. Must be a power of 2 & >= 512.
    pub sector_size: usize,
    /// Total number of addressable sectors on this device.
    pub sector_count: u64,
}

pub trait BlockDevice {
    fn size(&self) -> BlockDeviceSize;

    fn make_buf(&self, sector_count: usize) -> Box<[u8]> {
        vec![0u8; sector_count * self.size().sector_size].into_boxed_slice()
    }

    /// Read sectors at `lba` into `buf`.
    /// `buf.len()` must be a multiple of `self.sector_size()`.
    fn read_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;

    /// Write sectors at `lba` from `buf`.
    /// `buf.len()` must be a multiple of `self.sector_size()`.
    fn write_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;

    fn zero_sectors<'a>(
        &'a self,
        lba: u64,
        sector_count: u64,
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(default_zero_sectors(self, lba, sector_count))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;
}

async fn default_zero_sectors<D: ?Sized + BlockDevice>(
    device: &D,
    lba: u64,
    sector_count: u64,
) -> IoResult<()> {
    let zbuf = device.make_buf(sector_count as usize);
    device.write_sectors(lba, &zbuf).await
}
