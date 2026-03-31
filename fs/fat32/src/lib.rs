#![no_std]
#![feature(ptr_metadata)]
#![feature(const_cmp)]
#![feature(const_trait_impl)]
#![feature(const_option_ops)]

use {
    crate::{
        bpb::BiosParameterBlock,
        fat::{Fat, FatType},
        format::disk_table,
        fsinfo::FSInfo,
        media::MediaType,
    },
    alloc::{boxed::Box, sync::Arc},
    block::BlockDevice,
    core::sync::atomic::{AtomicUsize, Ordering},
    io::{IoError, IoResult},
    log::trace,
    sync::{AsyncMutex, AsyncMutexGuard},
};

extern crate alloc;

pub mod bpb;
pub mod dirent;
pub mod format;
pub mod fsinfo;
pub mod media;

mod fat;
mod random;

pub struct FatVolume {
    geometry: FatGeometry,
    fat: Arc<AsyncMutex<Fat>>,
}

pub struct FatFile {
    fat: Arc<AsyncMutex<Fat>>,
    geometry: FatGeometry,
    device: Arc<dyn BlockDevice>,

    write: bool,
    cluster_first: usize,
    file_size: Arc<AtomicUsize>,

    // Cursor state
    pos: usize,
    cluster_current: usize,
    cluster_buf: AsyncMutex<Box<[u8]>>,
    cluster_dirty: bool,
}

pub struct FormatOptions {
    pub label: Option<[u8; 11]>,
    pub id: Option<u32>,
    pub media: Option<MediaType>,
    pub full_zero: bool,
}

#[derive(Debug, Clone, Copy)]
struct FatGeometry {
    fat_type: FatType,
    data_cluster_count: usize,
    cluster_pg_count: usize,
    data_region_pg_first: usize,

    root_cluster: usize,      // FAT32 only
    root_dir_pg_first: usize, // FAT16 only
    root_entry_count: usize,  // FAT16 only
}

impl FatVolume {
    pub async fn format(device: Arc<dyn BlockDevice>, options: FormatOptions) -> IoResult<Self> {
        let device_dim = device.dimensions();

        if device_dim.page_size != 512 {
            // TODO: the rest of the code would probably work, I just need to make sure it does
            todo!()
        }

        let fat_type = disk_table::format_type(device_dim.page_count);

        // TODO: support formatting into FAT12
        if fat_type == FatType::Fat12 {
            return Err(IoError::Unsupported);
        }

        let fat32 = fat_type == FatType::Fat32;

        let id = options.id.unwrap_or(random::gen_u32());
        let label = options.label.unwrap_or(*b"NO NAME    ");
        let media = options.media.unwrap_or(MediaType::Fixed);

        let mut bpb_buf = device.make_buf(1);
        let bpb = BiosParameterBlock::from_raw_mut(&mut bpb_buf);
        bpb.format(device_dim, id, label, media);

        let data_cluster_count = bpb.data_cluster_count();
        let fat_pg_first = bpb.fat_pg_first();
        let fat_pg_count = bpb.fat_pg_count();
        let fat_count = bpb.fat_count();
        let cluster_pg_count = bpb.cluster_pg_count();
        let root_dir_pg_count = bpb.root_dir_pg_count();
        let data_region_pg_first = bpb.data_region_pg_first();
        let root_cluster = bpb.root_cluster();
        let root_dir_pg_first = bpb.root_dir_pg_first();
        let root_entry_count = bpb.root_entry_count();

        if options.full_zero {
            device.full_zero().await?;
        } else {
            let header_pg_count = if fat32 {
                // 32 reserved(BPB + FSInfo + backups + lots of empty space) + FATs + root_dir first cluster
                32 + fat_count * fat_pg_count + cluster_pg_count
            } else {
                // FAT16
                // BPB + FATs + root directory
                1 + fat_count * fat_pg_count + root_dir_pg_count
            };

            device.zero_pages(0, header_pg_count).await?;
        }

        device.write_pages(0, &bpb_buf).await?;

        if fat32 {
            device.write_pages(6, &bpb_buf).await?;
        }

        if fat32 {
            let mut fsinfo_buf = device.make_buf(1);

            let fsinfo = FSInfo::from_raw_mut(&mut fsinfo_buf);
            fsinfo.format();

            device.write_pages(1, &fsinfo_buf).await?;
            device.write_pages(7, &fsinfo_buf).await?;
        }

        let mut fat = Fat::alloc(
            fat_type,
            device_dim.page_size,
            fat_pg_first,
            fat_pg_count,
            data_cluster_count,
        );
        fat.set_media(media);
        fat.set_eoc();

        fat.set_entry(2, fat_type.eoc_mark_min());

        fat.flush(device).await?;

        trace!("FAT format success!");

        Ok(Self {
            geometry: FatGeometry {
                fat_type,
                data_cluster_count,
                cluster_pg_count,
                data_region_pg_first,
                root_cluster,
                root_dir_pg_first,
                root_entry_count,
            },
            fat: Arc::new(AsyncMutex::new(fat)),
        })
    }
}

impl FatFile {
    pub async fn read(&self, buf: &mut [u8]) -> IoResult<()> {
        if self.pos >= buf.len() {
            return Err(IoError::Eof);
        }
        if self.pos + buf.len() > self.size() {
            return Err(IoError::OutOfBounds);
        }

        let fat = self.fat.lock().await;

        let device_dim = self.device.dimensions();
        let cluster_size = self.geometry.cluster_pg_count * device_dim.page_size;

        let original_pos = self.pos;

        while self.pos - original_pos < buf.len() {
            let cluster_idx = self.pos / cluster_size;
            let cluster_pos = self.pos % cluster_size;

            let next_cluster = fat.cluster_follow(self.cluster_current);
        }

        todo!()
    }

    pub async fn write(&self, buf: &[u8]) -> IoResult<()> {
        todo!()
    }

    pub async fn seek(&self, pos: usize) -> IoResult<()> {
        if pos > self.size() {
            return Err(IoError::OutOfBounds);
        }
        let device_dim = self.device.dimensions();
        let cluster_size = self.geometry.cluster_pg_count * device_dim.page_size;

        let cluster_idx = pos / cluster_size;

        if cluster_idx == 0 {
            todo!()
            // self.pos = pos;
            // return self.load_cluster(self.cluster_first as usize).await;
        }

        let fat = self.fat.lock().await;

        todo!()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn size(&self) -> usize {
        self.file_size.load(Ordering::Relaxed)
    }

    pub async fn set_size(&self, new_size: usize) -> IoResult<()> {
        todo!()
    }

    pub async fn flush(&self) -> IoResult<()> {
        self.fat.lock().await.flush(self.device.clone()).await?;
        self.device.flush().await
    }

    async fn load_cluster(&mut self, index: usize) -> IoResult<AsyncMutexGuard<'_, Box<[u8]>>> {
        if self.cluster_current == index {
            Ok(self.cluster_buf.lock().await)
        } else {
            let mut cluster_buf = self.cluster_buf.lock().await;
            if self.cluster_dirty && self.write {
                self.device
                    .write_pages(
                        self.geometry.data_region_pg_first
                            + self.cluster_current * self.geometry.cluster_pg_count,
                        &cluster_buf,
                    )
                    .await?;
            }
            self.device
                .read_pages(
                    self.geometry.data_region_pg_first
                        + index * self.geometry.cluster_pg_count,
                    &mut cluster_buf,
                )
                .await?;
            self.cluster_dirty = false;
            self.cluster_current = index;
            Ok(cluster_buf)
        }
    }
}
