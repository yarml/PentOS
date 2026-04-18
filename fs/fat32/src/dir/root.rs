use core::pin::Pin;

use fs::FileBackend;

use {
    alloc::{boxed::Box, sync::Arc},
    block::BlockDevice,
    io::{IoError, IoResult},
};

pub struct FatRootBackend {
    device: Arc<dyn BlockDevice>,
    region_pg_first: usize,
    region_pg_count: usize,
    page_size: usize,
}

impl FatRootBackend {
    pub fn new(
        device: Arc<dyn BlockDevice>,
        region_pg_first: usize,
        region_pg_count: usize,
    ) -> Self {
        let page_size = device.dimensions().page_size;
        Self {
            device,
            region_pg_first,
            region_pg_count,
            page_size,
        }
    }

    fn region_size(&self) -> usize {
        self.region_pg_count * self.page_size
    }

    async fn resize_impl(&mut self, new_size: usize) -> IoResult<()> {
        // The fixed-size root cannot grow or shrink. We accept the
        // no-op call (so that callers that defensively call resize
        // with the current size don't fail), and reject anything else
        // with NoSpace, which is the natural error for "the directory
        // is full" surfaced by `FatDirectory::create_*`.
        if new_size == self.region_size() {
            Ok(())
        } else {
            Err(IoError::NoSpace)
        }
    }

    async fn read_chunk_impl(&mut self, chunk_index: usize, buf: &mut [u8]) -> IoResult<()> {
        if buf.len() != self.page_size {
            return Err(IoError::InvalidInput);
        }
        if chunk_index >= self.region_pg_count {
            return Err(IoError::OutOfBounds);
        }
        self.device
            .read_pages(self.region_pg_first + chunk_index, buf)
            .await
    }

    async fn write_chunk_impl(&mut self, chunk_index: usize, buf: &[u8]) -> IoResult<()> {
        if buf.len() != self.page_size {
            return Err(IoError::InvalidInput);
        }
        if chunk_index >= self.region_pg_count {
            return Err(IoError::OutOfBounds);
        }
        self.device
            .write_pages(self.region_pg_first + chunk_index, buf)
            .await
    }
}

impl FileBackend for FatRootBackend {
    fn chunk_size(&self) -> usize {
        self.page_size
    }

    fn size(&self) -> usize {
        self.region_size()
    }

    fn resize<'a>(
        &'a mut self,
        new_size: usize,
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.resize_impl(new_size))
    }

    fn read_chunk<'a>(
        &'a mut self,
        chunk_index: usize,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.read_chunk_impl(chunk_index, buf))
    }

    fn write_chunk<'a>(
        &'a mut self,
        chunk_index: usize,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.write_chunk_impl(chunk_index, buf))
    }
}
