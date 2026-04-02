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

pub trait FileBackend {
    fn chunk_size(&self) -> usize;

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

    fn size(&self) -> usize;
    fn resize<'a>(
        &'a mut self,
        new_size: usize,
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + Send + 'a>>;

    fn make_chunk_buf(&self) -> Box<[u8]> {
        vec![0u8; self.chunk_size()].into_boxed_slice()
    }
}

pub struct File {
    backend: AsyncMutex<Box<dyn FileBackend + Send>>,
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
    file: Arc<File>,
    index: usize,
    dirty: AtomicBool,
    data: AsyncMutex<Box<[u8]>>,
}

pub struct ChunkGuard<'chunk> {
    mutex_guard: AsyncMutexGuard<'chunk, Box<[u8]>>,
    dirty: &'chunk AtomicBool,
}

impl File {
    pub(crate) async fn from_backend(backend: Box<dyn FileBackend + Send>) -> Arc<Self> {
        let chunk_size = backend.chunk_size();
        let size = backend.size();
        Arc::new(Self {
            backend: AsyncMutex::new(backend),
            cache: AsyncMutex::new(BTreeMap::new()),
            chunk_size,
            size: AtomicUsize::new(size),
        })
    }
}

impl File {
    pub fn open(self: &Arc<Self>) -> OpenFile {
        OpenFile {
            file: self.clone(),
            chunk_size: self.chunk_size,
            position: 0,
        }
    }
}

impl OpenFile {
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
}

impl File {
    pub async fn flush(&self) -> IoResult<()> {
        let cache = self.cache.lock().await;
        for chunk in cache
            .values()
            .filter(|chunk| chunk.dirty.load(Ordering::Relaxed))
        {
            chunk.flush().await?;
        }
        Ok(())
    }
}

impl OpenFile {
    pub async fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if self.position + buf.len() > self.file.size.load(Ordering::Relaxed) {
            return Err(IoError::Eof);
        }

        let chunk_index = self.chunk_index();
        let chunk_offset = self.chunk_offset();

        let chunk = self.file.get_chunk(chunk_index).await?;
        let data = chunk.lock().await;

        Ok(self.copy_chunk_part(&data, buf, chunk_offset))
    }
    pub async fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        if self.position + buf.len() > self.file.size.load(Ordering::Relaxed) {
            return Err(IoError::Eof);
        }

        let chunk_index = self.chunk_index();
        let chunk_offset = self.chunk_offset();

        let chunk = self.file.get_chunk(chunk_index).await?;
        let mut data = chunk.lock().await;

        Ok(self.copy_chunk_part(buf, &mut data, chunk_offset))
    }
}

impl File {
    async fn get_chunk(self: &Arc<Self>, chunk_index: usize) -> IoResult<Arc<Chunk>> {
        let mut cache = self.cache.lock().await;
        if let Some(cached_chunk) = cache.get(&chunk_index) {
            return Ok(cached_chunk.clone());
        }

        let mut backend = self.backend.lock().await;
        let mut chunk_buf = backend.make_chunk_buf();
        backend.read_chunk(chunk_index, &mut chunk_buf).await?;

        let chunk = Arc::new(Chunk {
            file: self.clone(),
            index: chunk_index,
            dirty: AtomicBool::new(false),
            data: AsyncMutex::new(chunk_buf),
        });

        cache.insert(chunk_index, chunk.clone());
        Ok(chunk)
    }
}

impl Chunk {
    async fn lock(&self) -> ChunkGuard<'_> {
        ChunkGuard {
            mutex_guard: self.data.lock().await,
            dirty: &self.dirty,
        }
    }

    async fn flush(&self) -> IoResult<()> {
        let mut backend = self.file.backend.lock().await;
        let data = self.data.lock().await;
        backend.write_chunk(self.index, &data).await
    }
}

impl OpenFile {
    const fn chunk_index(&self) -> usize {
        self.position / self.chunk_size
    }
    const fn chunk_offset(&self) -> usize {
        self.position % self.chunk_size
    }

    fn copy_chunk_part(&self, src: &[u8], dst: &mut [u8], chunk_offset: usize) -> usize {
        let remaining_bytes = self.chunk_size - chunk_offset;
        let copy_amount = usize::min(remaining_bytes, dst.len());
        dst[..copy_amount].copy_from_slice(&src[chunk_offset..(chunk_offset + copy_amount)]);
        copy_amount
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
        self.dirty.store(false, Ordering::Relaxed);
        &mut self.mutex_guard
    }
}
