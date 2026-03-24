#![no_std]

extern crate alloc;

use {
    alloc::{boxed::Box, collections::btree_map::BTreeMap, sync::Arc, vec},
    block::{BlockDevice, BlockDeviceSize},
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
    sectors: AsyncMutex<BTreeMap<u64, Arc<CachedSector>>>,
    size: BlockDeviceSize,
    get_time: fn() -> usize,
}

pub struct CachedSector {
    device: Arc<AsyncMutex<dyn BlockDevice + Send>>,
    dirty: AtomicBool,
    data: AsyncMutex<Box<[u8]>>,

    lba: u64,

    /// How many times this sector has been accessed (we count the number of times it has been locked)
    /// will be used to sort which cached sectors can be evicted first (low usage)
    access_count: AtomicUsize,

    /// Moment in time when this cached sector started existing
    cache_instant_ms: AtomicUsize,

    /// Moment in time when this cached sector stopped being used the last time (time since unlock)
    last_access_ms: AtomicUsize,

    get_time: fn() -> usize,
}

pub struct CachedSectorGuard<'sector> {
    mutex_guard: AsyncMutexGuard<'sector, Box<[u8]>>,
    dirty: &'sector AtomicBool,
    last_access_ms: &'sector AtomicUsize,
    get_time: fn() -> usize,
}

impl BlockCache {
    pub fn create<D: BlockDevice + Send + 'static>(
        device: D,
        time_accessor: fn() -> usize,
    ) -> Self {
        let size = device.size();
        Self {
            device: Arc::new(AsyncMutex::new(device)),
            sectors: AsyncMutex::new(BTreeMap::new()),
            size,
            get_time: time_accessor,
        }
    }
}

impl BlockCache {
    pub fn size(&self) -> BlockDeviceSize {
        self.size
    }

    pub async fn get_sector(&self, lba: u64) -> IoResult<Arc<CachedSector>> {
        let mut sectors = self.sectors.lock().await;
        if let Some(cached_sector) = sectors.get(&lba) {
            return IoResult::Ok(cached_sector.clone());
        }
        let device = self.device.lock().await;
        let size = device.size();
        if size.sector_count < lba {
            return IoResult::Err(IoError::OutOfBounds);
        }
        let sector_size = size.sector_size;
        let data = vec![0u8; sector_size];
        let mut data = data.into_boxed_slice();
        device.read_sectors(lba, &mut data).await?;

        let current_time = (self.get_time)();

        let cached_sector = CachedSector {
            device: self.device.clone(),
            dirty: AtomicBool::new(false),
            data: AsyncMutex::new(data),
            lba,
            access_count: AtomicUsize::new(0),
            cache_instant_ms: AtomicUsize::new(current_time),
            last_access_ms: AtomicUsize::new(current_time),
            get_time: self.get_time,
        };
        let cached_sector = Arc::new(cached_sector);

        sectors.insert(lba, cached_sector.clone());
        IoResult::Ok(cached_sector)
    }

    async fn flush_impl(&self) -> IoResult<()> {
        let sectors = self.sectors.lock().await;
        for sector in sectors
            .values()
            .filter(|sector| sector.dirty.load(Ordering::Relaxed))
        {
            sector.flush().await?;
        }
        Ok(())
    }
}

impl CachedSector {
    pub async fn flush(&self) -> IoResult<()> {
        let device = self.device.lock().await;
        let data = self.data.lock().await;
        let result = device.write_sectors(self.lba, &data).await;
        if result.is_ok() {
            self.dirty.store(false, Ordering::Relaxed);
        }
        result
    }

    pub async fn lock(&self) -> CachedSectorGuard<'_> {
        let mutex_guard = self.data.lock().await;
        self.access_count.fetch_add(1, Ordering::Relaxed);
        CachedSectorGuard {
            mutex_guard,
            dirty: &self.dirty,
            last_access_ms: &self.last_access_ms,
            get_time: self.get_time,
        }
    }
}

impl Deref for CachedSectorGuard<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.mutex_guard
    }
}

impl DerefMut for CachedSectorGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty.store(true, Ordering::Relaxed);
        &mut self.mutex_guard
    }
}

impl Drop for CachedSectorGuard<'_> {
    fn drop(&mut self) {
        self.last_access_ms
            .store((self.get_time)(), Ordering::Relaxed);
    }
}

// TODO: figure out a way to flush a cached sector when it drops
// problem is flushing is async, and drop is sync
impl Drop for CachedSector {
    fn drop(&mut self) {
        if self.dirty.load(Ordering::Relaxed) {
            panic!("dropping dirty cached sector!")
        }
    }
}

impl BlockDevice for BlockCache {
    fn size(&self) -> BlockDeviceSize {
        self.size
    }

    fn read_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(read_sectors(self, lba, buf))
    }

    fn write_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(write_sectors(self, lba, buf))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.flush_impl())
    }
}

async fn read_sectors(cache: &BlockCache, lba: u64, buf: &mut [u8]) -> IoResult<()> {
    let sector_size = cache.size.sector_size;

    assert!(buf.len().is_multiple_of(sector_size));
    let sector_count = (buf.len() / sector_size) as u64;

    for i in 0..sector_count {
        let lba = lba + i;
        let sector_lock = cache.get_sector(lba).await?;
        let sector = sector_lock.lock().await;
        buf[(i as usize) * sector_size..(i as usize + 1) * sector_size].copy_from_slice(&sector);
    }

    Ok(())
}
async fn write_sectors(cache: &BlockCache, lba: u64, buf: &[u8]) -> IoResult<()> {
    let sector_size = cache.size.sector_size;

    assert!(buf.len().is_multiple_of(sector_size));
    let sector_count = (buf.len() / sector_size) as u64;

    for i in 0..sector_count {
        let lba = lba + i;
        let sector_lock = cache.get_sector(lba).await?;
        let mut sector = sector_lock.lock().await;
        sector.copy_from_slice(&buf[(i as usize) * sector_size..(i as usize + 1) * sector_size]);
    }

    Ok(())
}
