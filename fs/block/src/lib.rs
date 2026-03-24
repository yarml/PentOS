#![no_std]
extern crate alloc;

use {
    alloc::{boxed::Box, vec},
    core::pin::Pin,
    io::IoResult,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockDeviceDimensions {
    /// Block size in bytes. Whether physical or logical depends on the device.
    pub page_size: usize,
    /// Total number of addressable pages on this device.
    pub page_count: u64,

    pub optimal_transfer_size: Option<usize>,
}

pub trait BlockDevice {
    fn dimensions(&self) -> BlockDeviceDimensions;

    fn make_buf(&self, page_count: usize) -> Box<[u8]> {
        vec![0u8; page_count * self.dimensions().page_size].into_boxed_slice()
    }

    /// Read pages at `pg` into `buf`.
    /// `buf.len()` must be a multiple of `self.dimensions().page_size`.
    fn read_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;

    /// Write pages at `pg` from `buf`.
    /// `buf.len()` must be a multiple of `self.dimensions().page_size`.
    fn write_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;

    fn zero_pages<'a>(
        &'a self,
        pg: u64,
        page_count: u64,
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(default_zero_pages(self, pg, page_count))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>>;
}

async fn default_zero_pages<D: ?Sized + BlockDevice>(
    device: &D,
    pg: u64,
    page_count: u64,
) -> IoResult<()> {
    let zbuf = device.make_buf(page_count as usize);
    device.write_pages(pg, &zbuf).await
}
