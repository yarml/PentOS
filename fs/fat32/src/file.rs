use {
    crate::{FatGeometry, fat::Fat},
    alloc::{boxed::Box, sync::Arc, vec::Vec},
    block::BlockDevice,
    core::pin::Pin,
    fs::file::FileBackend,
    io::{IoError, IoResult},
    sync::AsyncMutex,
};

pub struct FatFileBackend {
    geometry: FatGeometry,
    device: Arc<dyn BlockDevice>,

    fat: Arc<AsyncMutex<Fat>>,

    size: usize,

    /// Fast (index) -> (FAT index) of clusters of this file
    clusters: Vec<usize>,
}

impl FatFileBackend {
    fn get_pg(&self, cluster: usize) -> usize {
        self.geometry.data_region_pg_first + self.geometry.cluster_pg_count * cluster
    }
}

impl FatFileBackend {
    async fn resize_impl(&mut self, new_size: usize) -> IoResult<()> {
        if new_size == self.size {
            return Ok(());
        }

        let chunk_size = self.chunk_size();

        let old_chunk_count = self.size.div_ceil(chunk_size);
        let new_chunk_count = new_size.div_ceil(chunk_size);

        if new_chunk_count > old_chunk_count {
            let diff = new_chunk_count - old_chunk_count;
            self.grow(diff).await?;
        } else if old_chunk_count > new_chunk_count {
            let diff = old_chunk_count - new_chunk_count;
            self.shrink(diff).await?;
        }

        self.size = new_size;

        Ok(())
    }

    async fn read_chunk_impl(&mut self, chunk_index: usize, buf: &mut [u8]) -> IoResult<()> {
        if !buf.len().is_multiple_of(self.chunk_size()) {
            return Err(IoError::InvalidInput);
        }
        let cluster = *self.clusters.get(chunk_index).ok_or(IoError::OutOfBounds)?;
        let pg = self.get_pg(cluster);
        self.device.read_pages(pg, buf).await
    }

    async fn write_chunk_impl(&mut self, chunk_index: usize, buf: &[u8]) -> IoResult<()> {
        if !buf.len().is_multiple_of(self.chunk_size()) {
            return Err(IoError::InvalidInput);
        }
        let cluster = *self.clusters.get(chunk_index).ok_or(IoError::OutOfBounds)?;
        let pg = self.get_pg(cluster);
        self.device.write_pages(pg, buf).await
    }

    async fn grow(&mut self, chunks: usize) -> IoResult<()> {
        let cluster_count_t0 = self.clusters.len();
        self.clusters.reserve(chunks);
        let mut fat = self.fat.lock().await;

        for _ in 0..chunks {
            let Some(cluster) = fat.cluster_alloc() else {
                break;
            };
            self.clusters.push(cluster);
        }

        if self.clusters.len() != cluster_count_t0 + chunks {
            for cluster in self.clusters.drain(cluster_count_t0..) {
                fat.cluster_free(cluster);
            }
            Err(IoError::NoSpace)
        } else {
            for (index, &cluster) in self.clusters.iter().enumerate().skip(cluster_count_t0) {
                if index > 0 {
                    let prev_cluster = self.clusters[index - 1];
                    fat.set_entry(prev_cluster, cluster);
                }
            }
            // No need to mark last entry as EOC, cluster_alloc makes them EOC by default
            Ok(())
        }
    }

    async fn shrink(&mut self, chunks: usize) -> IoResult<()> {
        let chunks = usize::min(chunks, self.clusters.len());
        let mut fat = self.fat.lock().await;

        for cluster in self.clusters.drain(self.clusters.len() - chunks..) {
            fat.cluster_free(cluster);
        }

        if let Some(&last_cluster) = self.clusters.last() {
            fat.make_eoc(last_cluster);
        }
        Ok(())
    }
}

impl FileBackend for FatFileBackend {
    fn chunk_size(&self) -> usize {
        self.geometry.cluster_pg_count * self.device.dimensions().page_size
    }
    fn size(&self) -> usize {
        self.size
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
