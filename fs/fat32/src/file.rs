use {
    crate::{FatGeometry, fat::Fat},
    alloc::{boxed::Box, sync::Arc, vec::Vec},
    block::BlockDevice,
    config::fs::fat32::FILE_CLUSTER_LOOKAHEAD,
    core::pin::Pin,
    fs::file::FileBackend,
    io::{IoError, IoResult},
    sync::AsyncMutex,
};

pub struct FatFileBackend {
    geometry: FatGeometry,
    device: Arc<dyn BlockDevice>,

    /// # Invariant
    /// Only None if size is 0
    cluster_first: Option<usize>,

    fat: Arc<AsyncMutex<Fat>>,

    size: usize,
    /// Fast index-FAT index of clusters of this file
    cluster_index_buf: Vec<u32>,
}

impl FatFileBackend {
    fn cluster_count(&self) -> usize {
        self.size.div_ceil(self.chunk_size())
    }

    async fn get_cluster(&mut self, cluster_index: usize) -> IoResult<usize> {
        if cluster_index < self.cluster_index_buf.len() {
            return Ok(self.cluster_index_buf[cluster_index] as usize);
        }

        if self.cluster_index_buf.is_empty() {
            self.cluster_index_buf.reserve(FILE_CLUSTER_LOOKAHEAD);
            // can unwrap as size is not zero per the first check:
            // cluster_index < self.cluster_count() => size != 0
            self.cluster_index_buf
                .push(self.cluster_first.unwrap() as u32);
        } else {
            self.cluster_index_buf
                .reserve(cluster_index - self.cluster_index_buf.len() - 1 + FILE_CLUSTER_LOOKAHEAD);
        }

        let mut current = *self.cluster_index_buf.last().unwrap() as usize;
        let mut index = self.cluster_index_buf.len() - 1;

        let fat = self.fat.lock().await;

        while index < cluster_index {
            current = fat.cluster_follow(current).ok_or(IoError::OutOfBounds)?;
            self.cluster_index_buf.push(current as u32);
            index += 1;
        }

        let target = current;

        for _ in 0..FILE_CLUSTER_LOOKAHEAD {
            let Some(next) = fat.cluster_follow(current) else {
                break;
            };
            current = next;
            self.cluster_index_buf.push(current as u32);
        }

        Ok(target)
    }

    fn get_pg(&self, cluster: usize) -> usize {
        self.geometry.data_region_pg_first + self.geometry.cluster_pg_count * cluster
    }
}

impl FatFileBackend {
    async fn read_chunk_impl(&mut self, chunk_index: usize, buf: &mut [u8]) -> IoResult<()> {
        if !buf.len().is_multiple_of(self.chunk_size()) {
            return Err(IoError::InvalidInput);
        }
        let cluster = self.get_cluster(chunk_index).await?;
        let pg = self.get_pg(cluster);
        self.device.read_pages(pg, buf).await
    }

    async fn write_chunk_impl(&mut self, chunk_index: usize, buf: &[u8]) -> IoResult<()> {
        if !buf.len().is_multiple_of(self.chunk_size()) {
            return Err(IoError::InvalidInput);
        }
        let cluster = self.get_cluster(chunk_index).await?;
        let pg = self.get_pg(cluster);
        self.device.write_pages(pg, buf).await
    }

    async fn resize_impl(&mut self, new_size: usize) -> IoResult<()> {
        if new_size == self.size {
            return Ok(());
        }

        let current_cluster_count = self.cluster_count();
        let new_cluster_count = new_size.div_ceil(self.chunk_size());

        if new_cluster_count == current_cluster_count {
            self.size = new_size;
            return Ok(());
        }

        if new_cluster_count > current_cluster_count {
            let mut new_clusters = self
                .allow_many_clusters(new_cluster_count - current_cluster_count)
                .await?;

            let current_cluster_count = if self.cluster_first.is_none() {
                self.cluster_first = Some(new_clusters.pop().unwrap());
                1
            } else {
                current_cluster_count
            };
            let diff = new_cluster_count - current_cluster_count;

            let mut current = self.get_cluster(current_cluster_count - 1).await?;
            let mut fat = self.fat.lock().await;
            self.cluster_index_buf.reserve(diff);
            for _ in 0..diff {
                let next = new_clusters.pop().unwrap();
                self.cluster_index_buf.push(next as u32);
                fat.set_entry(current, next);
                current = next;
            }
            self.size = new_size;
            Ok(())
        } else {
            // new_cluster_count < current_cluster_count

            // Making sure to load the cluster indices of all the clusters we're about to remove
            self.get_cluster(current_cluster_count - 1).await?;

            let mut fat = self.fat.lock().await;

            for cluster in self.cluster_index_buf.drain(new_cluster_count..) {
                let cluster = cluster as usize;
                fat.cluster_free(cluster);
            }

            if let Some(cluster) = self.cluster_index_buf.last() {
                let cluster = *cluster as usize;
                fat.make_eoc(cluster);
            }

            self.size = new_size;
            Ok(())
        }
    }

    async fn allow_many_clusters(&mut self, count: usize) -> IoResult<Vec<usize>> {
        let mut clusters = Vec::with_capacity(count);
        let mut fat = self.fat.lock().await;

        for _ in 0..count {
            let Some(cluster) = fat.cluster_alloc() else {
                break;
            };
            clusters.push(cluster);
        }

        if clusters.len() != count {
            for cluster in clusters.drain(..) {
                fat.cluster_free(cluster);
            }
            Err(IoError::NoSpace)
        } else {
            clusters.reverse();
            Ok(clusters)
        }
    }
}

impl FileBackend for FatFileBackend {
    fn chunk_size(&self) -> usize {
        self.geometry.cluster_pg_count * self.device.dimensions().page_size
    }
    fn size(&self) -> usize {
        self.size
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

    fn resize<'a>(
        &'a mut self,
        new_size: usize,
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.resize_impl(new_size))
    }
}
