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
    alloc::{borrow::ToOwned, boxed::Box, string::String, vec::Vec},
    block::BlockDevice,
    core::ops::RangeInclusive,
    io::{IoError, IoResult},
};

pub struct GptDisk {
    device: Box<dyn BlockDevice>,
    guid: Guid,
    partlist: Vec<GptPartition>,

    partlist_cap: usize,
    partlist_dirty: bool,

    p_cache: HeaderCache,
    b_cache: HeaderCache,

    lba_usable: RangeInclusive<u64>,
}

pub struct GptPartition {
    guid: Guid,
    type_guid: Guid,
    name: String,
    lba_start: u64,
    lba_end: u64,
}

// Simple
impl GptDisk {
    pub fn lba_usable(&self) -> RangeInclusive<u64> {
        self.lba_usable.clone()
    }
    pub fn disk_guid(&self) -> Guid {
        self.guid
    }
    pub fn done(self) -> Box<dyn BlockDevice> {
        self.device
    }
}

impl GptPartition {
    pub fn lba_range(&self) -> RangeInclusive<u64> {
        self.lba_start..=self.lba_end
    }
}

// Partition stuff
impl GptDisk {
    pub fn partitions(&self) -> &[GptPartition] {
        &self.partlist
    }

    pub fn add_partition(
        &mut self,
        type_guid: Guid,
        name: &str,
        lba_start: u64,
        lba_end: u64,
    ) -> IoResult<()> {
        if self.partlist.len() >= self.partlist_cap {
            return Err(IoError::NoSpace);
        }
        if lba_start > lba_end
            || lba_start < *self.lba_usable.start()
            || lba_end > *self.lba_usable.end()
        {
            return Err(IoError::InvalidInput);
        }
        for entry in &self.partlist {
            if entry.lba_range().contains(&lba_start) || entry.lba_range().contains(&lba_end) {
                return Err(IoError::AlreadyExists);
            }
        }
        self.partlist.push(GptPartition {
            guid: Guid::gen_v4(),
            type_guid,
            name: name.to_owned(),
            lba_start,
            lba_end,
        });
        self.partlist_dirty = true;
        Ok(())
    }

    pub fn remove_partition(&mut self, guid: Guid) -> IoResult<()> {
        let mut index = None;
        for (i, entry) in self.partlist.iter().enumerate() {
            if entry.guid == guid {
                index = Some(i);
                break;
            }
        }
        if let Some(index) = index {
            self.partlist.remove(index);
            self.partlist_dirty = true;
            Ok(())
        } else {
            Err(IoError::NotFound)
        }
    }
}

// Heavy duty procedures
impl GptDisk {
    pub async fn open<D: BlockDevice + 'static>(device: D) -> IoResult<Self> {
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
                });
            }
        }

        let lba_usable = p_header.usable_lba();
        let partlist_cap = p_header.partlist_cap();

        let disk_guid = p_header.disk_guid();

        Ok(Self {
            device: Box::new(device),
            guid: disk_guid,
            partlist: entries,
            partlist_cap,
            partlist_dirty: false,
            lba_usable,
            p_cache: HeaderCache {
                lba: 1,
                header: p_header_buf,
                partlist: p_partlist_buf,
            },
            b_cache: HeaderCache {
                lba: b_header_lba,
                header: b_header_buf,
                partlist: b_partlist_buf,
            },
        })
    }

    pub async fn format<D: BlockDevice + 'static>(
        device: D,
        options: FormatOptions,
    ) -> IoResult<Self> {
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
            device: Box::new(device),
            guid: disk_guid,
            partlist: Vec::new(),
            partlist_cap,
            partlist_dirty: false,

            lba_usable,
            p_cache: HeaderCache {
                lba: 1,
                header: p_header_buf,
                partlist: p_partlist_buf,
            },
            b_cache: HeaderCache {
                lba: lbaz,
                header: b_header_buf,
                partlist: b_partlist_buf,
            },
        })
    }

    pub async fn flush(&mut self) -> IoResult<()> {
        if self.partlist_dirty {
            self.p_cache.partlist.fill(0);

            let p_partlist = self.p_cache.partlist_mut();
            for (i, part) in self.partlist.iter().enumerate() {
                p_partlist[i] = PartitionEntry::new(
                    Some(part.guid),
                    part.type_guid,
                    part.lba_start,
                    part.lba_end,
                    &part.name,
                );
            }

            let new_partlist_crc32 = crypto::crc32(&self.p_cache.partlist);

            let p_header = self.p_cache.header_mut();
            let b_header = self.b_cache.header_mut();

            p_header.update_partlist_crc32(new_partlist_crc32);
            b_header.update_partlist_crc32(new_partlist_crc32);

            self.device
                .write_sectors(
                    *p_header
                        .partlist_lba(self.device.size().sector_size)
                        .start(),
                    &self.p_cache.partlist,
                )
                .await?;
            self.device
                .write_sectors(
                    *b_header
                        .partlist_lba(self.device.size().sector_size)
                        .start(),
                    // we're deliberatly using the primary's cached partlist
                    // The backup cache does not track partlist since it should always
                    // be equal to the primary's
                    &self.p_cache.partlist,
                )
                .await?;

            self.device
                .write_sectors(self.p_cache.lba, &self.p_cache.header)
                .await?;
            self.device
                .write_sectors(self.b_cache.lba, &self.b_cache.header)
                .await?;
            self.partlist_dirty = false;
        }
        self.device.flush().await
    }
}
