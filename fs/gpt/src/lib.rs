#![no_std]
#![feature(const_trait_impl)]
#![feature(const_cmp)]
#![feature(ptr_metadata)]

extern crate alloc;

pub mod format;
pub mod guid;

mod mbr;

use {
    crate::{format::GPTHeader, guid::Guid, mbr::MasterBootRecord},
    alloc::{boxed::Box, vec::Vec},
    block::{BlockDevice, BlockDeviceSize},
    block_cache::BlockCache,
    core::pin::Pin,
    io::{IoError, IoResult},
};

#[derive(Clone, Copy)]
pub struct FormatOptions {
    /// Anything below 128 will be ignored and 128 will be the actual capacity allocated
    partition_capacity: Option<u32>,
    /// If not specified, a Guid will be generated
    guid: Option<Guid>,

    full_zero: bool,
}

#[derive(Debug, Clone)]
pub struct PartitionEntry {
    pub type_guid: Guid,
    pub partition_guid: Guid,
    pub start_lba: u64,
    pub end_lba: u64, // inclusive
    pub attributes: u64,
    pub name: [u16; 36], // UTF-16LE, null-terminated
}

pub struct GptDisk {
    device: BlockCache,
    // entries: Vec<PartitionEntry>,
}

impl GptDisk {
    pub async fn open(device: BlockCache) -> IoResult<Self> {
        {
            let lb_lock = device.get_sector(1).await?;
            let mut lb = lb_lock.lock().await;
            let header = GPTHeader::interpret_gpt_header(&mut lb);
            if !header.check(1, &device).await? {
                return Err(IoError::Corrupted);
            }
        }
        Ok(Self { device })
    }

    pub async fn format(device: BlockCache, options: FormatOptions) -> IoResult<Self> {
        let device_size = device.size();
        assert!(device_size.sector_size >= 512);
        let lb0_lock = device.get_sector(0).await?;
        let lb1_lock = device.get_sector(1).await?;
        let lbz_lock = device.get_sector(device_size.sector_count - 1).await?;
        {
            let mut lb0 = lb0_lock.lock().await;
            let mbr = MasterBootRecord::interpret_mbr(&mut lb0);
            mbr.set_protective(device_size.sector_count);
        }
        let (usable_sectors, primary_partition_list) = {
            let mut lb1 = lb1_lock.lock().await;
            let gpt_header = GPTHeader::interpret_gpt_header(&mut lb1);
            gpt_header.format(device_size, true, options);
            (
                gpt_header.usable_lba(),
                gpt_header.partition_list_lba(device_size.sector_size),
            )
        };
        let backup_partition_list = {
            let mut lbz = lbz_lock.lock().await;
            let backup_gpt_header = GPTHeader::interpret_gpt_header(&mut lbz);
            backup_gpt_header.format(device_size, false, options);
            backup_gpt_header.partition_list_lba(device_size.sector_size)
        };

        // zero primary & backup partition list
        for lba in primary_partition_list {
            let lba_lock = device.get_sector(lba).await?;
            let mut lba = lba_lock.lock().await;
            lba.fill(0);
        }
        for lba in backup_partition_list {
            let lba_lock = device.get_sector(lba).await?;
            let mut lb = lba_lock.lock().await;
            lb.fill(0);
        }

        if options.full_zero {
            for lba in usable_sectors {
                let lb_lock = device.get_sector(lba).await?;
                let mut lb = lb_lock.lock().await;
                lb.fill(0);
            }
        }

        Ok(Self {
            device,
            // entries: todo!(),
        })
    }

    pub fn partitions(&self) -> &[PartitionEntry] {
        todo!()
    }

    pub fn add_partition(
        &mut self,
        type_guid: Guid,
        name: &str,
        start_lba: u64,
        end_lba: u64,
    ) -> IoResult<usize> {
        todo!()
    }

    pub fn remove_partition(&mut self, index: usize) -> IoResult<()> {
        todo!()
    }

    pub fn flush(&mut self) -> IoResult<()> {
        todo!()
    }

    pub fn partition_view(&mut self, index: usize) -> IoResult<PartitionView<'_>> {
        todo!()
    }
}

pub struct PartitionView<'a> {
    device: &'a mut Box<dyn BlockDevice>,
    start_lba: u64,
    end_lba: u64, // inclusive
}

impl<'a> BlockDevice for PartitionView<'a> {
    fn size(&self) -> BlockDeviceSize {
        BlockDeviceSize {
            sector_count: self.end_lba - self.start_lba + 1,
            sector_size: self.device.size().sector_size,
        }
    }

    fn read_sectors<'b>(
        &'b mut self,
        lba: u64,
        buf: &'b mut [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'b>> {
        todo!()
    }

    fn write_sectors<'b>(
        &'b mut self,
        lba: u64,
        buf: &'b [u8],
    ) -> Pin<Box<dyn Future<Output = IoResult<()>> + 'b>> {
        todo!()
    }

    // fn read_sector(&mut self, lba: u64, buf: &mut [u8]) -> IoResult<()> {
    //     let real_lba = lba + self.start_lba;
    //     if real_lba > self.end_lba {
    //         return IoResult::Err(IoError::OutOfBounds);
    //     }
    //     self.device.read_sector(real_lba, buf)
    // }

    // fn write_sector(&mut self, lba: u64, buf: &[u8]) -> IoResult<()> {
    //     let real_lba = lba + self.start_lba;
    //     if real_lba > self.end_lba {
    //         return IoResult::Err(IoError::OutOfBounds);
    //     }
    //     self.device.write_sector(real_lba, buf)
    // }
}
