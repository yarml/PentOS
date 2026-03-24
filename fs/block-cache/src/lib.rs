#![no_std]

extern crate alloc;

use {
    alloc::{boxed::Box, collections::btree_map::BTreeMap, sync::Arc, vec},
    block::{BlockDevice, BlockDeviceDimensions},
    core::{
        ops::{Deref, DerefMut},
        pin::Pin,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    io::{IoError, IoResult},
    sync::{AsyncMutex, AsyncMutexGuard},
};

pub struct BlockCache {
    device: Arc<AsyncMutex<dyn BlockDevice + Send>>,
    pages: AsyncMutex<BTreeMap<u64, Arc<CachedPage>>>,
    size: BlockDeviceDimensions,
    get_time: fn() -> usize,
}

pub struct CachedPage {
    device: Arc<AsyncMutex<dyn BlockDevice + Send>>,
    dirty: AtomicBool,
    data: AsyncMutex<Box<[u8]>>,

    pg: u64,

    /// How many times this page has been accessed (we count the number of times it has been locked)
    /// will be used to sort which cached pages can be evicted first (low usage)
    access_count: AtomicUsize,

    /// Moment in time when this cached page started existing
    cache_instant_ms: AtomicUsize,

    /// Moment in time when this cached page stopped being used the last time (time since unlock)
    last_access_ms: AtomicUsize,

    get_time: fn() -> usize,
}

pub struct CachedPageGuard<'page> {
    mutex_guard: AsyncMutexGuard<'page, Box<[u8]>>,
    dirty: &'page AtomicBool,
    last_access_ms: &'page AtomicUsize,
    get_time: fn() -> usize,
}

impl BlockCache {
    pub fn create<D: BlockDevice + Send + 'static>(
        device: D,
        time_accessor: fn() -> usize,
    ) -> Self {
        let size = device.dimensions();
        Self {
            device: Arc::new(AsyncMutex::new(device)),
            pages: AsyncMutex::new(BTreeMap::new()),
            size,
            get_time: time_accessor,
        }
    }
}

impl BlockCache {
    pub fn size(&self) -> BlockDeviceDimensions {
        self.size
    }

    pub async fn get_page(&self, pg: u64) -> IoResult<Arc<CachedPage>> {
        let mut pages = self.pages.lock().await;
        if let Some(cached_page) = pages.get(&pg) {
            return IoResult::Ok(cached_page.clone());
        }
        let device = self.device.lock().await;
        let size = device.dimensions();
        if size.page_count < pg {
            return IoResult::Err(IoError::OutOfBounds);
        }
        let page_size = size.page_size;
        let data = vec![0u8; page_size];
        let mut data = data.into_boxed_slice();
        device.read_pages(pg, &mut data).await?;

        let current_time = (self.get_time)();

        let cached_page = CachedPage {
            device: self.device.clone(),
            dirty: AtomicBool::new(false),
            data: AsyncMutex::new(data),
            pg,
            access_count: AtomicUsize::new(0),
            cache_instant_ms: AtomicUsize::new(current_time),
            last_access_ms: AtomicUsize::new(current_time),
            get_time: self.get_time,
        };
        let cached_page = Arc::new(cached_page);

        pages.insert(pg, cached_page.clone());
        IoResult::Ok(cached_page)
    }

    async fn flush_impl(&self) -> IoResult<()> {
        let pages = self.pages.lock().await;
        for page in pages
            .values()
            .filter(|page| page.dirty.load(Ordering::Relaxed))
        {
            page.flush().await?;
        }
        Ok(())
    }
}

impl CachedPage {
    pub async fn flush(&self) -> IoResult<()> {
        let device = self.device.lock().await;
        let data = self.data.lock().await;
        let result = device.write_pages(self.pg, &data).await;
        if result.is_ok() {
            self.dirty.store(false, Ordering::Relaxed);
        }
        result
    }

    pub async fn lock(&self) -> CachedPageGuard<'_> {
        let mutex_guard = self.data.lock().await;
        self.access_count.fetch_add(1, Ordering::Relaxed);
        CachedPageGuard {
            mutex_guard,
            dirty: &self.dirty,
            last_access_ms: &self.last_access_ms,
            get_time: self.get_time,
        }
    }
}

impl Deref for CachedPageGuard<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.mutex_guard
    }
}

impl DerefMut for CachedPageGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty.store(true, Ordering::Relaxed);
        &mut self.mutex_guard
    }
}

impl Drop for CachedPageGuard<'_> {
    fn drop(&mut self) {
        self.last_access_ms
            .store((self.get_time)(), Ordering::Relaxed);
    }
}

// TODO: figure out a way to flush a cached page when it drops
// problem is flushing is async, and drop is sync
impl Drop for CachedPage {
    fn drop(&mut self) {
        if self.dirty.load(Ordering::Relaxed) {
            panic!("dropping dirty cached page!")
        }
    }
}

impl BlockDevice for BlockCache {
    fn dimensions(&self) -> BlockDeviceDimensions {
        self.size
    }

    fn read_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(read_pages_impl(self, pg, buf))
    }

    fn write_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(write_pages_impl(self, pg, buf))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.flush_impl())
    }
}

async fn read_pages_impl(cache: &BlockCache, pg: u64, buf: &mut [u8]) -> IoResult<()> {
    let page_size = cache.size.page_size;

    assert!(buf.len().is_multiple_of(page_size));
    let page_count = (buf.len() / page_size) as u64;

    for i in 0..page_count {
        let pg = pg + i;
        let page_lock = cache.get_page(pg).await?;
        let page = page_lock.lock().await;
        buf[(i as usize) * page_size..(i as usize + 1) * page_size].copy_from_slice(&page);
    }

    Ok(())
}
async fn write_pages_impl(cache: &BlockCache, pg: u64, buf: &[u8]) -> IoResult<()> {
    let page_size = cache.size.page_size;

    assert!(buf.len().is_multiple_of(page_size));
    let page_count = (buf.len() / page_size) as u64;

    for i in 0..page_count {
        let pg = pg + i;
        let page_lock = cache.get_page(pg).await?;
        let mut page = page_lock.lock().await;
        page.copy_from_slice(&buf[(i as usize) * page_size..(i as usize + 1) * page_size]);
    }

    Ok(())
}
