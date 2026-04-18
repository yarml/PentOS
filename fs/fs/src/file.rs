use {
    alloc::{boxed::Box, collections::btree_map::BTreeMap, sync::Arc, vec},
    core::{
        ops::{Deref, DerefMut},
        pin::Pin,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    io::{IoError, IoResult},
    sync::{AsyncMutex, AsyncMutexGuard},
};

pub trait FileBackend: Send {
    fn chunk_size(&self) -> usize;

    fn size(&self) -> usize;
    fn resize<'a>(
        &'a mut self,
        new_size: usize,
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>>;

    fn read_chunk<'a>(
        &'a mut self,
        chunk_index: usize,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>>;

    fn write_chunk<'a>(
        &'a mut self,
        chunk_index: usize,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>>;

    /// Called by [`File::flush`] AFTER all dirty chunks have been written
    /// back. Backends that need to update external metadata implement this.
    fn flush<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn make_chunk_buf(&self) -> Box<[u8]> {
        vec![0u8; self.chunk_size()].into_boxed_slice()
    }
}

pub struct File {
    backend: AsyncMutex<Box<dyn FileBackend>>,
    cache: AsyncMutex<BTreeMap<usize, Arc<Chunk>>>,
    chunk_size: usize,
    size: AtomicUsize,
}

pub struct OpenFile {
    file: Arc<File>,
    chunk_size: usize,
    position: usize,
}

pub struct Chunk {
    index: usize,
    dirty: AtomicBool,
    data: AsyncMutex<Box<[u8]>>,
}

pub struct ChunkGuard<'chunk> {
    mutex_guard: AsyncMutexGuard<'chunk, Box<[u8]>>,
    dirty: &'chunk AtomicBool,
}

impl File {
    pub fn from_backend(backend: Box<dyn FileBackend>) -> Arc<Self> {
        let chunk_size = backend.chunk_size();
        let size = backend.size();
        Arc::new(Self {
            backend: AsyncMutex::new(backend),
            cache: AsyncMutex::new(BTreeMap::new()),
            chunk_size,
            size: AtomicUsize::new(size),
        })
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::Acquire)
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn open(self: &Arc<Self>) -> OpenFile {
        OpenFile {
            file: self.clone(),
            chunk_size: self.chunk_size,
            position: 0,
        }
    }

    pub async fn flush(&self) -> IoResult<()> {
        let dirty_chunks: alloc::vec::Vec<Arc<Chunk>> = {
            let cache = self.cache.lock().await;
            cache
                .values()
                .filter(|c| c.dirty.load(Ordering::Acquire))
                .cloned()
                .collect()
        };
        for chunk in dirty_chunks {
            chunk.flush_with(&self.backend).await?;
        }

        let mut backend = self.backend.lock().await;
        backend.flush().await?;
        Ok(())
    }
}

impl OpenFile {
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn len(&self) -> usize {
        self.file.size()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn seek(&mut self, position: usize) {
        self.position = position;
    }

    pub async fn read_all(&mut self, mut buf: &mut [u8]) -> IoResult<()> {
        while !buf.is_empty() {
            let read = self.read(buf).await?;
            buf = &mut buf[read..];
        }
        Ok(())
    }

    pub async fn write_all(&mut self, mut buf: &[u8]) -> IoResult<()> {
        while !buf.is_empty() {
            let written = self.write(buf).await?;
            buf = &buf[written..];
        }
        Ok(())
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let size = self.file.size();
        if self.position >= size {
            return Err(IoError::Eof);
        }

        let chunk_index = self.chunk_index();
        let chunk_offset = self.chunk_offset();

        let chunk_remaining = self.chunk_size - chunk_offset;
        let absolute_remaining = size - self.position;

        let chunk = self.file.get_chunk(chunk_index).await?;
        let data = chunk.lock().await;

        let copy_amount = chunk_remaining.min(absolute_remaining).min(buf.len());
        buf[..copy_amount].copy_from_slice(&data[chunk_offset..(chunk_offset + copy_amount)]);
        self.position += copy_amount;
        Ok(copy_amount)
    }

    pub async fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let old_size = self.file.size();
        let min_size = self.position + buf.len();
        if min_size > old_size {
            self.resize(min_size).await?;
        }

        let chunk_index = self.chunk_index();
        let chunk_offset = self.chunk_offset();

        let chunk_first_byte = chunk_index * self.chunk_size;
        let chunk = if chunk_first_byte >= old_size {
            self.file.get_or_insert_zero_chunk(chunk_index).await?
        } else {
            self.file.get_chunk(chunk_index).await?
        };
        let mut data = chunk.lock().await;

        let copy_amount = (self.chunk_size - chunk_offset).min(buf.len());
        data[chunk_offset..chunk_offset + copy_amount].copy_from_slice(&buf[..copy_amount]);
        self.position += copy_amount;
        Ok(copy_amount)
    }

    pub async fn resize(&mut self, new_size: usize) -> IoResult<()> {
        let old_size = self.file.size();

        let mut backend = self.file.backend.lock().await;
        backend.resize(new_size).await?;

        self.file.size.store(new_size, Ordering::Release);
        drop(backend);

        let chunk_size = self.chunk_size;

        if new_size > old_size {
            let tail_offset = old_size % chunk_size;

            if tail_offset > 0 {
                let last_old_chunk = old_size / chunk_size;
                let chunk = self.file.get_chunk(last_old_chunk).await?;
                let mut data = chunk.lock().await;
                let chunk_start = last_old_chunk * chunk_size;
                let zero_end = chunk_size.min(new_size - chunk_start);

                for b in &mut data[tail_offset..zero_end] {
                    *b = 0;
                }
            }
        } else if new_size < old_size {
            let max_chunk = new_size.div_ceil(chunk_size);
            let mut cache = self.file.cache.lock().await;
            cache.retain(|&i, _| i < max_chunk);
        }

        Ok(())
    }

    const fn chunk_index(&self) -> usize {
        self.position / self.chunk_size
    }
    const fn chunk_offset(&self) -> usize {
        self.position % self.chunk_size
    }
}

impl File {
    async fn get_chunk(self: &Arc<Self>, chunk_index: usize) -> IoResult<Arc<Chunk>> {
        {
            let cache = self.cache.lock().await;
            if let Some(c) = cache.get(&chunk_index) {
                return Ok(c.clone());
            }
        }

        let chunk_buf = {
            let mut backend = self.backend.lock().await;
            let mut buf = backend.make_chunk_buf();
            backend.read_chunk(chunk_index, &mut buf).await?;
            buf
        };

        let mut cache = self.cache.lock().await;

        // In case some other thread loaded the chunk in the small moment we dropped
        // the lock
        if let Some(existing) = cache.get(&chunk_index) {
            return Ok(existing.clone());
        }

        let chunk = Arc::new(Chunk {
            index: chunk_index,
            dirty: AtomicBool::new(false),
            data: AsyncMutex::new(chunk_buf),
        });

        cache.insert(chunk_index, chunk.clone());

        Ok(chunk)
    }

    async fn get_or_insert_zero_chunk(
        self: &Arc<Self>,
        chunk_index: usize,
    ) -> IoResult<Arc<Chunk>> {
        let mut cache = self.cache.lock().await;
        if let Some(c) = cache.get(&chunk_index) {
            return Ok(c.clone());
        }

        let buf = vec![0u8; self.chunk_size].into_boxed_slice();
        let chunk = Arc::new(Chunk {
            index: chunk_index,
            dirty: AtomicBool::new(true),
            data: AsyncMutex::new(buf),
        });
        cache.insert(chunk_index, chunk.clone());
        Ok(chunk)
    }
}

impl Chunk {
    pub async fn lock(&self) -> ChunkGuard<'_> {
        ChunkGuard {
            mutex_guard: self.data.lock().await,
            dirty: &self.dirty,
        }
    }

    async fn flush_with(&self, backend: &AsyncMutex<Box<dyn FileBackend>>) -> IoResult<()> {
        let mut backend = backend.lock().await;
        let data = self.data.lock().await;
        backend.write_chunk(self.index, &data).await?;
        self.dirty.store(false, Ordering::Release);
        Ok(())
    }
}

impl Deref for ChunkGuard<'_> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.mutex_guard
    }
}

impl DerefMut for ChunkGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty.store(true, Ordering::Release);
        &mut self.mutex_guard
    }
}
