#![no_std]
#![feature(ptr_metadata)]
#![feature(const_cmp)]
#![feature(const_trait_impl)]

use {
    crate::{
        bpb::BiosParameterBlock,
        fat::{Fat, FatType},
        format::disk_table,
        fsinfo::FSInfo,
        media::MediaType,
    },
    alloc::sync::Arc,
    block::BlockDevice,
    io::{IoError, IoResult},
    log::trace,
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
    fat_type: FatType,

    cluster_count: usize,

    fat: Fat,
}

pub struct FormatOptions {
    pub label: Option<[u8; 11]>,
    pub id: Option<u32>,
    pub media: Option<MediaType>,
    pub full_zero: bool,
}

impl FatVolume {
    pub async fn format(device: Arc<dyn BlockDevice>, options: FormatOptions) -> IoResult<Self> {
        let device_dim = device.dimensions();

        if device_dim.page_size != 512 {
            // TODO: the rest of the code would probably work, I just need to make sure it does
            todo!()
        }

        let fat_type = disk_table::format_type(device_dim.page_count as usize);

        // TODO: support formatting into FAT12
        if fat_type == FatType::Fat12 {
            return Err(IoError::Unsupported);
        }

        let id = options.id.unwrap_or(random::gen_u32());
        let label = options.label.unwrap_or(*b"NO NAME    ");
        let media = options.media.unwrap_or(MediaType::Fixed);

        let mut bpb_buf = device.make_buf(1);
        let bpb = BiosParameterBlock::from_raw_mut(&mut bpb_buf);
        bpb.format(device_dim, id, label, media);

        let cluster_count = bpb.data_cluster_count();
        let fat_pg_first = bpb.fat_pg_first();
        let fat_pg_count = bpb.fat_pg_count();
        let fat_count = bpb.fat_count();
        let cluster_pg_count = bpb.cluster_pg_count();

        if options.full_zero {
            device.full_zero().await?;
        } else {
            let header_pg_count = if fat_type == FatType::Fat32 {
                // 32 reserved(BPB + FSInfo + backups + lots of empty space) + FATs + root_dir first cluster
                32 + fat_count * fat_pg_count + cluster_pg_count
            } else {
                // FAT16
                // BPB + FATs + root directory
                1 + fat_count * fat_pg_count + (512 * 32) / device_dim.page_size
            };

            device.zero_pages(0, header_pg_count as u64).await?;
        }

        device.write_pages(0, &bpb_buf).await?;

        if fat_type == FatType::Fat32 {
            device.write_pages(6, &bpb_buf).await?;
        }

        if fat_type == FatType::Fat32 {
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
            fat_count,
        );
        fat.set_media(media);
        fat.set_eoc();
        fat.flush(device).await?;

        trace!("FAT format success!");

        Ok(Self {
            fat_type,
            cluster_count,
            fat,
        })
    }
}
