//! FAT32 [`FileBackend`] implementation.

use {
    crate::{
        FatGeometry,
        dirent::{DIRENT_SIZE, ShortDirEntry},
        fat::Fat,
    },
    alloc::{boxed::Box, sync::Arc, vec::Vec},
    block::BlockDevice,
    core::pin::Pin,
    fs::file::{File, FileBackend},
    io::{IoError, IoResult},
    sync::AsyncMutex,
};

pub struct FatFileBackend {
    geometry: FatGeometry,
    device: Arc<dyn BlockDevice>,
    fat: Arc<AsyncMutex<Fat>>,
    size: usize,
    clusters: Vec<usize>,
    parent_entry: Option<ParentEntry>,
    entry_dirty: bool,
}

struct ParentEntry {
    storage: Arc<File>,
    sfn_slot: usize,
}

impl FatFileBackend {
    pub async fn open(
        geometry: FatGeometry,
        device: Arc<dyn BlockDevice>,
        fat: Arc<AsyncMutex<Fat>>,
        first_cluster: Option<usize>,
        size: usize,
    ) -> IoResult<Self> {
        let clusters = match first_cluster {
            None => Vec::new(),
            Some(fc) => {
                let f = fat.lock().await;

                let mut chain = Vec::new();
                let mut cur = Some(fc);
                let cap = geometry.data_cluster_count;

                while let Some(c) = cur {
                    if chain.len() > cap {
                        return Err(IoError::Corrupted);
                    }

                    chain.push(c);
                    cur = f.cluster_follow(c);
                }
                chain
            }
        };

        Ok(Self {
            geometry,
            device,
            fat,
            size,
            clusters,
            parent_entry: None,
            entry_dirty: false,
        })
    }

    pub fn new_empty(
        geometry: FatGeometry,
        device: Arc<dyn BlockDevice>,
        fat: Arc<AsyncMutex<Fat>>,
    ) -> Self {
        Self {
            geometry,
            device,
            fat,
            size: 0,
            clusters: Vec::new(),
            parent_entry: None,
            entry_dirty: false,
        }
    }

    pub fn with_parent_entry(mut self, parent_storage: Arc<File>, sfn_slot: usize) -> Self {
        self.parent_entry = Some(ParentEntry {
            storage: parent_storage,
            sfn_slot,
        });
        self
    }

    pub fn first_cluster(&self) -> Option<usize> {
        self.clusters.first().copied()
    }

    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    fn cluster_to_pg(&self, cluster: usize) -> usize {
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
            self.grow(new_chunk_count - old_chunk_count).await?;
        } else if old_chunk_count > new_chunk_count {
            self.shrink(old_chunk_count - new_chunk_count).await?;
        }

        self.size = new_size;

        self.entry_dirty = true;
        Ok(())
    }

    async fn grow(&mut self, chunks: usize) -> IoResult<()> {
        let cluster_count_t0 = self.clusters.len();
        self.clusters.reserve(chunks);
        let mut fat = self.fat.lock().await;

        for _ in 0..chunks {
            let Some(c) = fat.cluster_alloc() else { break };
            self.clusters.push(c);
        }
        if self.clusters.len() != cluster_count_t0 + chunks {
            for c in self.clusters.drain(cluster_count_t0..) {
                fat.cluster_free(c);
            }
            return Err(IoError::NoSpace);
        }

        for i in cluster_count_t0..self.clusters.len() {
            if i == 0 {
                continue;
            }
            let prev = self.clusters[i - 1];
            let cur = self.clusters[i];
            fat.set_entry(prev, cur);
        }
        Ok(())
    }

    async fn shrink(&mut self, chunks: usize) -> IoResult<()> {
        let chunks = chunks.min(self.clusters.len());
        let mut fat = self.fat.lock().await;
        for c in self.clusters.drain(self.clusters.len() - chunks..) {
            fat.cluster_free(c);
        }
        if let Some(&last) = self.clusters.last() {
            fat.make_eoc(last);
        }
        Ok(())
    }

    async fn read_chunk_impl(&mut self, chunk_index: usize, buf: &mut [u8]) -> IoResult<()> {
        if buf.len() != self.chunk_size() {
            return Err(IoError::InvalidInput);
        }
        let cluster = *self.clusters.get(chunk_index).ok_or(IoError::OutOfBounds)?;
        self.device
            .read_pages(self.cluster_to_pg(cluster), buf)
            .await
    }

    async fn write_chunk_impl(&mut self, chunk_index: usize, buf: &[u8]) -> IoResult<()> {
        if buf.len() != self.chunk_size() {
            return Err(IoError::InvalidInput);
        }
        let cluster = *self.clusters.get(chunk_index).ok_or(IoError::OutOfBounds)?;
        self.device
            .write_pages(self.cluster_to_pg(cluster), buf)
            .await
    }

    async fn flush_impl(&mut self) -> IoResult<()> {
        // Write the SFN entry

        if !self.entry_dirty {
            return Ok(());
        }
        let Some(parent) = &self.parent_entry else {
            self.entry_dirty = false;
            return Ok(());
        };

        let disk_cluster = match self.first_cluster() {
            Some(idx) => idx + 2,
            None => 0,
        };
        let size_u32 = u32::try_from(self.size).map_err(|_| IoError::InvalidInput)?;

        // Read the SFN slot.
        let mut buf = [0u8; DIRENT_SIZE];
        let mut open = parent.storage.open();

        open.seek(parent.sfn_slot * DIRENT_SIZE);
        open.read_all(&mut buf).await?;
        let mut sfn = ShortDirEntry::from_bytes(&buf);

        sfn.set_cluster(disk_cluster);
        sfn.file_size = size_u32;

        sfn.write_to(&mut buf);

        open.seek(parent.sfn_slot * DIRENT_SIZE);
        open.write_all(&buf).await?;

        parent.storage.flush().await?;

        self.entry_dirty = false;
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

    fn flush<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(self.flush_impl())
    }
}
