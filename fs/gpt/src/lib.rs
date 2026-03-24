#![no_std]
#![feature(const_trait_impl)]
#![feature(const_cmp)]
#![feature(ptr_metadata)]

extern crate alloc;

pub mod format;
pub mod guid;

mod cache;
mod mbr;

use {
    crate::{
        cache::HeaderCache,
        format::{FormatOptions, GptHeader, PartitionEntry},
        guid::Guid,
        mbr::MasterBootRecord,
    },
    alloc::{borrow::ToOwned, boxed::Box, string::String, sync::Arc, vec::Vec},
    block::{BlockDevice, BlockDeviceDimensions},
    core::{
        ops::RangeInclusive,
        pin::Pin,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    io::{IoError, IoResult},
    sync::AsyncMutex,
};

pub struct GptDisk {
    device: Arc<dyn BlockDevice>,
    guid: Guid,

    usable_pages: RangeInclusive<u64>,

    partlist_cap: usize,
    partlist_dirty: AtomicBool,
    partlist: AsyncMutex<Vec<GptPartition>>,

    p_cache: AsyncMutex<HeaderCache>,
    b_cache: AsyncMutex<HeaderCache>,
}

pub struct GptPartition {
    guid: Guid,
    type_guid: Guid,
    name: String,
    pg_start: u64,
    pg_end: u64,
    open_count: Arc<AtomicUsize>,
}

pub struct GptOpenPartition {
    device: Arc<dyn BlockDevice>,
    start_pg: u64,
    end_pg: u64,
    open_count: Arc<AtomicUsize>,
}

// Simple stuff
impl GptDisk {
    pub fn usable_pages(&self) -> RangeInclusive<u64> {
        self.usable_pages.clone()
    }
    pub fn disk_guid(&self) -> Guid {
        self.guid
    }
    pub fn done(self) -> Arc<dyn BlockDevice> {
        self.device
    }
}

impl GptPartition {
    pub fn pages(&self) -> RangeInclusive<u64> {
        self.pg_start..=self.pg_end
    }
}
impl GptOpenPartition {
    pub fn get_absolute_pg(&self, rel_pg: u64) -> Option<u64> {
        let abs_pg = rel_pg + self.start_pg;
        if abs_pg > self.end_pg {
            None
        } else {
            Some(abs_pg)
        }
    }
}

// Partition stuff
impl GptDisk {
    pub async fn add_partition(
        &self,
        type_guid: Guid,
        name: &str,
        pg_start: u64,
        pg_end: u64,
    ) -> IoResult<Guid> {
        let mut partlist = self.partlist.lock().await;

        if partlist.len() >= self.partlist_cap {
            return Err(IoError::NoSpace);
        }
        if pg_start > pg_end
            || pg_start < *self.usable_pages.start()
            || pg_end > *self.usable_pages.end()
        {
            return Err(IoError::InvalidInput);
        }

        if partlist.iter().any(|entry| {
            entry.pages().contains(&pg_start) || entry.pages().contains(&pg_end)
        }) {
            return Err(IoError::AlreadyExists);
        }

        let guid = Guid::gen_v4();

        partlist.push(GptPartition {
            guid,
            type_guid,
            name: name.to_owned(),
            pg_start,
            pg_end,
            open_count: Arc::new(AtomicUsize::new(0)),
        });
        self.partlist_dirty.store(true, Ordering::Relaxed);
        Ok(guid)
    }

    pub async fn remove_partition(&mut self, guid: Guid) -> IoResult<()> {
        let mut partlist = self.partlist.lock().await;

        let Some((index, entry)) = partlist
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.guid == guid)
        else {
            return Err(IoError::NotFound);
        };

        if entry
            .open_count
            .compare_exchange(0, usize::MAX, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return Err(IoError::InUse);
        }

        partlist.remove(index);
        self.partlist_dirty.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub async fn open_partition(&self, guid: Guid) -> IoResult<GptOpenPartition> {
        let partlist = self.partlist.lock().await;

        let Some((index, entry)) = partlist
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.guid == guid)
        else {
            return Err(IoError::NotFound);
        };
        if entry
            .open_count
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                // Some other thread called remove_partition in parallel
                // So now we consider the partition not found
                if current == usize::MAX {
                    None
                } else {
                    Some(current + 1)
                }
            })
            .is_err()
        {
            return Err(IoError::NotFound);
        }

        let partition = &partlist[index];

        Ok(GptOpenPartition {
            device: self.device.clone(),
            start_pg: partition.pg_start,
            end_pg: partition.pg_end,
            open_count: partition.open_count.clone(),
        })
    }
}

// Heavy duty procedures
impl GptDisk {
    pub async fn open(device: Arc<dyn BlockDevice>) -> IoResult<Self> {
        let page_size = device.dimensions().page_size;

        let mut p_header_buf = device.make_buf(1);
        let mut b_header_buf = device.make_buf(1);

        device.read_pages(1, &mut p_header_buf).await?;
        let p_header = GptHeader::from_raw_mut(&mut p_header_buf);

        let b_header_pg = p_header.alternate_pg();

        device.read_pages(b_header_pg, &mut b_header_buf).await?;
        let b_header = GptHeader::from_raw_mut(&mut b_header_buf);

        let p_partlist_pg = p_header.partlist_pg(page_size);
        let b_partlist_pg = b_header.partlist_pg(page_size);

        let mut p_partlist_buf = device.make_buf(p_partlist_pg.clone().count());
        let mut b_partlist_buf = device.make_buf(b_partlist_pg.clone().count());

        device
            .read_pages(*p_partlist_pg.start(), &mut p_partlist_buf)
            .await?;
        device
            .read_pages(*b_partlist_pg.start(), &mut b_partlist_buf)
            .await?;

        let p_partlist = PartitionEntry::from_raw_mut(&mut p_partlist_buf, p_header.partlist_cap());
        let b_partlist = PartitionEntry::from_raw_mut(&mut b_partlist_buf, b_header.partlist_cap());

        if !GptHeader::check(p_header, b_header, p_partlist, b_partlist) {
            return Err(IoError::Corrupted);
        }

        let mut entries = Vec::new();
        for entry in p_partlist {
            if entry.type_guid != Guid::NULL {
                entries.push(GptPartition {
                    type_guid: entry.type_guid,
                    guid: entry.guid,
                    pg_start: entry.pg_start,
                    pg_end: entry.pg_end,
                    name: entry.name(),
                    open_count: Arc::new(AtomicUsize::new(0)),
                });
            }
        }

        let usable_pages = p_header.usable_pages();
        let partlist_cap = p_header.partlist_cap();

        let disk_guid = p_header.disk_guid();

        Ok(Self {
            device,
            guid: disk_guid,
            partlist: AsyncMutex::new(entries),
            partlist_cap,
            partlist_dirty: AtomicBool::new(false),
            usable_pages,
            p_cache: AsyncMutex::new(HeaderCache {
                pg: 1,
                header: p_header_buf,
                partlist: p_partlist_buf,
            }),
            b_cache: AsyncMutex::new(HeaderCache {
                pg: b_header_pg,
                header: b_header_buf,
                partlist: b_partlist_buf,
            }),
        })
    }

    pub async fn format(device: Arc<dyn BlockDevice>, options: FormatOptions) -> IoResult<Self> {
        let device_dimensions = device.dimensions();
        assert!(device_dimensions.page_size >= 512);

        let last_pg = device_dimensions.page_count - 1;
        let mut page_buf = device.make_buf(1);

        // Protective MBR stuff
        {
            device.read_pages(0, &mut page_buf).await?;
            let mbr = MasterBootRecord::from_raw(&mut page_buf);
            mbr.set_protective(device_dimensions.page_count);
            device.write_pages(0, &page_buf).await?;
        }

        let mut p_header_buf = device.make_buf(1);
        let mut b_header_buf = device.make_buf(1);

        let p_header = GptHeader::from_raw_mut(&mut p_header_buf);
        let b_header = GptHeader::from_raw_mut(&mut b_header_buf);

        let disk_guid = options.guid.unwrap_or_else(Guid::gen_v4);

        p_header.format(device_dimensions, true, options.with_guid(disk_guid));
        b_header.format(device_dimensions, false, options.with_guid(disk_guid));

        let usable_pages = p_header.usable_pages();
        let partlist_cap = p_header.partlist_cap();

        let p_partlist_pg = p_header.partlist_pg(device_dimensions.page_size);
        let b_partlist_pg = b_header.partlist_pg(device_dimensions.page_size);

        device.write_pages(1, &p_header_buf).await?;
        device.write_pages(last_pg, &b_header_buf).await?;
        device
            .zero_pages(
                *p_partlist_pg.start(),
                p_partlist_pg.clone().count() as u64,
            )
            .await?;
        device
            .zero_pages(
                *b_partlist_pg.start(),
                b_partlist_pg.clone().count() as u64,
            )
            .await?;

        if options.full_zero {
            let range = (p_partlist_pg.end() + 1)..*b_partlist_pg.start();
            device
                .zero_pages(range.start, range.count() as u64)
                .await?;
        }

        let p_partlist_buf = device.make_buf(p_partlist_pg.count());
        let b_partlist_buf = device.make_buf(b_partlist_pg.count());

        Ok(Self {
            device,
            guid: disk_guid,
            partlist: AsyncMutex::new(Vec::new()),
            partlist_cap,
            partlist_dirty: AtomicBool::new(false),

            usable_pages,
            p_cache: AsyncMutex::new(HeaderCache {
                pg: 1,
                header: p_header_buf,
                partlist: p_partlist_buf,
            }),
            b_cache: AsyncMutex::new(HeaderCache {
                pg: last_pg,
                header: b_header_buf,
                partlist: b_partlist_buf,
            }),
        })
    }
}

impl Drop for GptOpenPartition {
    fn drop(&mut self) {
        self.open_count.fetch_sub(1, Ordering::Relaxed);
    }
}

// BlockDevice interfacing

impl GptDisk {
    async fn read_pages_impl(&self, pg: u64, buf: &mut [u8]) -> IoResult<()> {
        let page_count = (buf.len() / self.device.dimensions().page_size) as u64;
        let last_pg = pg + page_count - 1;
        if self.usable_pages.contains(&pg) && self.usable_pages.contains(&last_pg) {
            self.device.read_pages(pg, buf).await
        } else {
            Err(IoError::OutOfBounds)
        }
    }

    async fn write_pages_impl(&self, pg: u64, buf: &[u8]) -> IoResult<()> {
        let page_count = (buf.len() / self.device.dimensions().page_size) as u64;
        let last_pg = pg + page_count - 1;
        if self.usable_pages.contains(&pg) && self.usable_pages.contains(&last_pg) {
            self.device.write_pages(pg, buf).await
        } else {
            Err(IoError::OutOfBounds)
        }
    }

    async fn flush_impl(&self) -> IoResult<()> {
        if self
            .partlist_dirty
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            let mut p_cache = self.p_cache.lock().await;
            let mut b_cache = self.b_cache.lock().await;
            let partlist = self.partlist.lock().await;

            b_cache.partlist.fill(0);
            let b_partlist = b_cache.partlist_mut();

            for (i, part) in partlist.iter().enumerate() {
                b_partlist[i] = PartitionEntry::new(
                    Some(part.guid),
                    part.type_guid,
                    part.pg_start,
                    part.pg_end,
                    &part.name,
                );
            }

            let new_partlist_crc32 = crypto::crc32(&b_cache.partlist);

            let p_header = p_cache.header_mut();
            let b_header = b_cache.header_mut();

            p_header.update_partlist_crc32(new_partlist_crc32);
            b_header.update_partlist_crc32(new_partlist_crc32);

            // FIXME: if some of these writes succeed, and others fail,
            // The disk is left in an invalid GPT format
            // For now we at least attempt writing to the backup
            // before the primary, and we write the partlists
            // before the headers
            // FIXME: also currently, we have no way of making atomic operations
            // on devices, which we should support ASAP.
            // FIXME: swear this is the last, we keep the partlist_dirty set to false even on failure
            self.device
                .write_pages(
                    *b_header
                        .partlist_pg(self.device.dimensions().page_size)
                        .start(),
                    &b_cache.partlist,
                )
                .await?;
            self.device
                .write_pages(
                    *p_header
                        .partlist_pg(self.device.dimensions().page_size)
                        .start(),
                    // we're deliberatly using the backup's cached partlist
                    // The primary cache does not track partlist since it should always
                    // be equal to the backup's
                    &b_cache.partlist,
                )
                .await?;

            self.device
                .write_pages(b_cache.pg, &b_cache.header)
                .await?;
            self.device
                .write_pages(p_cache.pg, &p_cache.header)
                .await?;
        }
        self.device.flush().await
    }
}

impl GptOpenPartition {
    async fn read_pages_impl(&self, pg: u64, buf: &mut [u8]) -> IoResult<()> {
        let page_count = (buf.len() / self.device.dimensions().page_size) as u64;
        let last_pg = pg + page_count - 1;
        let pg = self.get_absolute_pg(pg).ok_or(IoError::OutOfBounds)?;
        self.get_absolute_pg(last_pg)
            .ok_or(IoError::OutOfBounds)?;

        self.device.read_pages(pg, buf).await
    }
    async fn write_pages_impl(&self, pg: u64, buf: &[u8]) -> IoResult<()> {
        let page_count = (buf.len() / self.device.dimensions().page_size) as u64;
        let last_pg = pg + page_count - 1;
        let pg = self.get_absolute_pg(pg).ok_or(IoError::OutOfBounds)?;
        self.get_absolute_pg(last_pg)
            .ok_or(IoError::OutOfBounds)?;

        self.device.write_pages(pg, buf).await
    }
    async fn flush_impl(&self) -> IoResult<()> {
        // FIXME: flush should take a range parameter for which pages to flush
        // So that a partition flushing can flush only its pages instead of
        // flushing the whole drive
        self.device.flush().await
    }
}

impl BlockDevice for GptOpenPartition {
    fn dimensions(&self) -> BlockDeviceDimensions {
        let device_dimensions = self.device.dimensions();
        BlockDeviceDimensions {
            page_size: device_dimensions.page_size,
            page_count: self.end_pg - self.start_pg + 1,
            optimal_transfer_size: device_dimensions.optimal_transfer_size
        }
    }

    fn read_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.read_pages_impl(pg, buf))
    }

    fn write_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.write_pages_impl(pg, buf))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.flush_impl())
    }
}

impl BlockDevice for GptDisk {
    fn dimensions(&self) -> BlockDeviceDimensions {
        self.device.dimensions()
    }

    fn read_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.read_pages_impl(pg, buf))
    }

    fn write_pages<'a>(
        &'a self,
        pg: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.write_pages_impl(pg, buf))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.flush_impl())
    }
}
