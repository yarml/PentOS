#![no_std]
extern crate alloc;

use {
    alloc::{boxed::Box, vec},
    core::pin::Pin,
    io::IoResult,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockDeviceDimensions {
    /// Logical block size in bytes.
    pub page_size: usize,
    /// Total number of addressable pages on this device.
    pub page_count: u64,

    /// Frame size of the physical media
    pub frame_size: Option<usize>,

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

    fn full_zero<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(default_full_zero(self))
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

async fn default_full_zero<D: ?Sized + BlockDevice>(device: &D) -> IoResult<()> {
    let device_dim = device.dimensions();
    let transfer_size = device_dim
        .optimal_transfer_size
        .unwrap_or(device_dim.page_size)
        .max(device_dim.page_size);
    let device_size = device_dim.page_size * device_dim.page_count as usize;
    let transfer_count = device_size / transfer_size;
    let rem_pg_count = (device_size % transfer_size) / device_dim.page_size; // likely 0

    let transfer_pg_count = transfer_size / device_dim.page_size;

    for i in 0..transfer_count {
        let pg = transfer_pg_count * i;
        device
            .zero_pages(pg as u64, transfer_pg_count as u64)
            .await?;
    }

    if rem_pg_count > 0 {
        let rem_pg_start = transfer_pg_count * transfer_count;
        for i in 0..rem_pg_count {
            let pg = device_dim.page_size * i + rem_pg_start;
            device.zero_pages(pg as u64, 1).await?;
        }
    }
    Ok(())
}
