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
    block::{BlockDevice, BlockDeviceSize},
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

    lba_usable: RangeInclusive<u64>,

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
    lba_start: u64,
    lba_end: u64,
    open_count: Arc<AtomicUsize>,
}

pub struct GptOpenPartition {
    device: Arc<dyn BlockDevice>,
    lba_start: u64,
    lba_end: u64,
    open_count: Arc<AtomicUsize>,
}

// Simple stuff
impl GptDisk {
    pub fn lba_usable(&self) -> RangeInclusive<u64> {
        self.lba_usable.clone()
    }
    pub fn disk_guid(&self) -> Guid {
        self.guid
    }
    pub fn done(self) -> Arc<dyn BlockDevice> {
        self.device
    }
}

impl GptPartition {
    pub fn lba_range(&self) -> RangeInclusive<u64> {
        self.lba_start..=self.lba_end
    }
}
impl GptOpenPartition {
    pub fn get_absolute_lba(&self, rel_lba: u64) -> Option<u64> {
        let absolute_lba = rel_lba + self.lba_start;
        if absolute_lba > self.lba_end {
            None
        } else {
            Some(absolute_lba)
        }
    }
}

// Partition stuff
impl GptDisk {
    pub async fn add_partition(
        &self,
        type_guid: Guid,
        name: &str,
        lba_start: u64,
        lba_end: u64,
    ) -> IoResult<Guid> {
        let mut partlist = self.partlist.lock().await;

        if partlist.len() >= self.partlist_cap {
            return Err(IoError::NoSpace);
        }
        if lba_start > lba_end
            || lba_start < *self.lba_usable.start()
            || lba_end > *self.lba_usable.end()
        {
            return Err(IoError::InvalidInput);
        }

        if partlist.iter().any(|entry| {
            entry.lba_range().contains(&lba_start) || entry.lba_range().contains(&lba_end)
        }) {
            return Err(IoError::AlreadyExists);
        }

        let guid = Guid::gen_v4();

        partlist.push(GptPartition {
            guid,
            type_guid,
            name: name.to_owned(),
            lba_start,
            lba_end,
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
            lba_start: partition.lba_start,
            lba_end: partition.lba_end,
            open_count: partition.open_count.clone(),
        })
    }
}

// Heavy duty procedures
impl GptDisk {
    pub async fn open(device: Arc<dyn BlockDevice>) -> IoResult<Self> {
        let sector_size = device.size().sector_size;

        let mut p_header_buf = device.make_buf(1);
        let mut b_header_buf = device.make_buf(1);

        device.read_sectors(1, &mut p_header_buf).await?;
        let p_header = GptHeader::from_raw_mut(&mut p_header_buf);

        let b_header_lba = p_header.alternate_lba();

        device.read_sectors(b_header_lba, &mut b_header_buf).await?;
        let b_header = GptHeader::from_raw_mut(&mut b_header_buf);

        let p_partlist_lba = p_header.partlist_lba(sector_size);
        let b_partlist_lba = b_header.partlist_lba(sector_size);

        let mut p_partlist_buf = device.make_buf(p_partlist_lba.clone().count());
        let mut b_partlist_buf = device.make_buf(b_partlist_lba.clone().count());

        device
            .read_sectors(*p_partlist_lba.start(), &mut p_partlist_buf)
            .await?;
        device
            .read_sectors(*b_partlist_lba.start(), &mut b_partlist_buf)
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
                    lba_start: entry.lba_start,
                    lba_end: entry.lba_end,
                    name: entry.name(),
                    open_count: Arc::new(AtomicUsize::new(0)),
                });
            }
        }

        let lba_usable = p_header.usable_lba();
        let partlist_cap = p_header.partlist_cap();

        let disk_guid = p_header.disk_guid();

        Ok(Self {
            device,
            guid: disk_guid,
            partlist: AsyncMutex::new(entries),
            partlist_cap,
            partlist_dirty: AtomicBool::new(false),
            lba_usable,
            p_cache: AsyncMutex::new(HeaderCache {
                lba: 1,
                header: p_header_buf,
                partlist: p_partlist_buf,
            }),
            b_cache: AsyncMutex::new(HeaderCache {
                lba: b_header_lba,
                header: b_header_buf,
                partlist: b_partlist_buf,
            }),
        })
    }

    pub async fn format(device: Arc<dyn BlockDevice>, options: FormatOptions) -> IoResult<Self> {
        let device_size = device.size();
        assert!(device_size.sector_size >= 512);

        let lbaz = device_size.sector_count - 1;
        let mut sector_buf = device.make_buf(1);

        // Protective MBR stuff
        {
            device.read_sectors(0, &mut sector_buf).await?;
            let mbr = MasterBootRecord::interpret_mbr(&mut sector_buf);
            mbr.set_protective(device_size.sector_count);
            device.write_sectors(0, &sector_buf).await?;
        }

        let mut p_header_buf = device.make_buf(1);
        let mut b_header_buf = device.make_buf(1);

        let p_header = GptHeader::from_raw_mut(&mut p_header_buf);
        let b_header = GptHeader::from_raw_mut(&mut b_header_buf);

        let disk_guid = options.guid.unwrap_or_else(Guid::gen_v4);

        p_header.format(device_size, true, options.with_guid(disk_guid));
        b_header.format(device_size, false, options.with_guid(disk_guid));

        let lba_usable = p_header.usable_lba();
        let partlist_cap = p_header.partlist_cap();

        let p_partlist_lba = p_header.partlist_lba(device_size.sector_size);
        let b_partlist_lba = b_header.partlist_lba(device_size.sector_size);

        device.write_sectors(1, &p_header_buf).await?;
        device.write_sectors(lbaz, &b_header_buf).await?;
        device
            .zero_sectors(
                *p_partlist_lba.start(),
                p_partlist_lba.clone().count() as u64,
            )
            .await?;
        device
            .zero_sectors(
                *b_partlist_lba.start(),
                b_partlist_lba.clone().count() as u64,
            )
            .await?;

        if options.full_zero {
            let range = (p_partlist_lba.end() + 1)..*b_partlist_lba.start();
            device
                .zero_sectors(range.start, range.count() as u64)
                .await?;
        }

        let p_partlist_buf = device.make_buf(p_partlist_lba.count());
        let b_partlist_buf = device.make_buf(b_partlist_lba.count());

        Ok(Self {
            device,
            guid: disk_guid,
            partlist: AsyncMutex::new(Vec::new()),
            partlist_cap,
            partlist_dirty: AtomicBool::new(false),

            lba_usable,
            p_cache: AsyncMutex::new(HeaderCache {
                lba: 1,
                header: p_header_buf,
                partlist: p_partlist_buf,
            }),
            b_cache: AsyncMutex::new(HeaderCache {
                lba: lbaz,
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
    async fn read_sectors_impl(&self, lba: u64, buf: &mut [u8]) -> IoResult<()> {
        let sector_count = (buf.len() / self.device.size().sector_size) as u64;
        let last_lba = lba + sector_count - 1;
        if self.lba_usable.contains(&lba) && self.lba_usable.contains(&last_lba) {
            self.device.read_sectors(lba, buf).await
        } else {
            Err(IoError::OutOfBounds)
        }
    }

    async fn write_sectors_impl(&self, lba: u64, buf: &[u8]) -> IoResult<()> {
        let sector_count = (buf.len() / self.device.size().sector_size) as u64;
        let last_lba = lba + sector_count - 1;
        if self.lba_usable.contains(&lba) && self.lba_usable.contains(&last_lba) {
            self.device.write_sectors(lba, buf).await
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
                    part.lba_start,
                    part.lba_end,
                    &part.name,
                );
            }

            let new_partlist_crc32 = crypto::crc32(&p_cache.partlist);

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
                .write_sectors(
                    *b_header
                        .partlist_lba(self.device.size().sector_size)
                        .start(),
                    &b_cache.partlist,
                )
                .await?;
            self.device
                .write_sectors(
                    *p_header
                        .partlist_lba(self.device.size().sector_size)
                        .start(),
                    // we're deliberatly using the backup's cached partlist
                    // The primary cache does not track partlist since it should always
                    // be equal to the backup's
                    &b_cache.partlist,
                )
                .await?;

            self.device
                .write_sectors(b_cache.lba, &b_cache.header)
                .await?;
            self.device
                .write_sectors(p_cache.lba, &p_cache.header)
                .await?;
        }
        self.device.flush().await
    }
}

impl GptOpenPartition {
    async fn read_sectors_impl(&self, lba: u64, buf: &mut [u8]) -> IoResult<()> {
        let sector_count = (buf.len() / self.device.size().sector_size) as u64;
        let last_lba = lba + sector_count - 1;
        let lba = self.get_absolute_lba(lba).ok_or(IoError::OutOfBounds)?;
        self.get_absolute_lba(last_lba)
            .ok_or(IoError::OutOfBounds)?;

        self.device.read_sectors(lba, buf).await
    }
    async fn write_sectors_impl(&self, lba: u64, buf: &[u8]) -> IoResult<()> {
        let sector_count = (buf.len() / self.device.size().sector_size) as u64;
        let last_lba = lba + sector_count - 1;
        let lba = self.get_absolute_lba(lba).ok_or(IoError::OutOfBounds)?;
        self.get_absolute_lba(last_lba)
            .ok_or(IoError::OutOfBounds)?;

        self.device.write_sectors(lba, buf).await
    }
    async fn flush_impl(&self) -> IoResult<()> {
        // FIXME: flush should take a range parameter for which LBAs to flush
        // So that a partition flushing can flush only its sectors instead of
        // flushing the whole drive
        self.device.flush().await
    }
}

impl BlockDevice for GptOpenPartition {
    fn size(&self) -> BlockDeviceSize {
        let device_size = self.device.size();
        BlockDeviceSize {
            sector_size: device_size.sector_size,
            sector_count: self.lba_end - self.lba_start + 1,
        }
    }

    fn read_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.read_sectors_impl(lba, buf))
    }

    fn write_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.write_sectors_impl(lba, buf))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.flush_impl())
    }
}

impl BlockDevice for GptDisk {
    fn size(&self) -> BlockDeviceSize {
        self.device.size()
    }

    fn read_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.read_sectors_impl(lba, buf))
    }

    fn write_sectors<'a>(
        &'a self,
        lba: u64,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.write_sectors_impl(lba, buf))
    }

    fn flush<'a>(&'a self) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'a>> {
        Box::pin(self.flush_impl())
    }
}
