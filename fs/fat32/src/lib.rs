#![no_std]
#![feature(ptr_metadata)]
#![feature(const_cmp)]
#![feature(const_trait_impl)]
#![feature(const_option_ops)]

use {
    crate::{
        bpb::BiosParameterBlock,
        dir::FatDirectory,
        fat::{Fat, FatType},
        format::disk_table,
        fsinfo::FSInfo,
        media::MediaType,
    },
    alloc::sync::Arc,
    block::BlockDevice,
    fs::dir::{Directory, Filesystem},
    io::{IoError, IoResult},
    sync::AsyncMutex,
};

extern crate alloc;

pub mod dir;
pub mod file;
pub mod media;

mod bpb;
mod dirent;
mod fat;
mod format;
mod fsinfo;
mod random;

pub struct FatVolume {
    device: Arc<dyn BlockDevice>,
    #[allow(dead_code)]
    geometry: FatGeometry,
    fat: Arc<AsyncMutex<Fat>>,
    root: Arc<FatDirectory>,
}

pub struct FormatOptions {
    pub label: Option<[u8; 11]>,
    pub id: Option<u32>,
    pub media: Option<MediaType>,
    pub full_zero: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct FatGeometry {
    pub fat_type: FatType,
    pub data_cluster_count: usize,
    pub cluster_pg_count: usize,
    pub data_region_pg_first: usize,

    pub root_cluster: usize,      // FAT32 only
    pub root_dir_pg_first: usize, // FAT16/12 only
    pub root_entry_count: usize,  // FAT16/12 only
}

impl FatVolume {
    pub async fn mount(device: Arc<dyn BlockDevice>) -> IoResult<Self> {
        let device_dim = device.dimensions();

        let mut bpb_buf = device.make_buf(1);
        device.read_pages(0, &mut bpb_buf).await?;

        // Per FAT spec: bpb_buf[510] == 0x55, bpb_buf[511] == 0xAA.
        if bpb_buf.len() < 512 || bpb_buf[510] != 0x55 || bpb_buf[511] != 0xAA {
            return Err(IoError::Corrupted);
        }

        let bpb = BiosParameterBlock::from_raw_mut(&mut bpb_buf);

        // TODO: i don't wanna think rn about about any page size other than 512
        if device_dim.page_size != 512 {
            return Err(IoError::Unsupported);
        }

        let fat_type = bpb.fat_type();

        // TODO: give FAT12 some care
        if fat_type == FatType::Fat12 {
            // Same constraint as `format`.
            return Err(IoError::Unsupported);
        }

        let geometry = FatGeometry {
            fat_type,
            data_cluster_count: bpb.data_cluster_count(),
            cluster_pg_count: bpb.cluster_pg_count(),
            data_region_pg_first: bpb.data_region_pg_first(),
            root_cluster: bpb.root_cluster(),
            root_dir_pg_first: bpb.root_dir_pg_first(),
            root_entry_count: bpb.root_entry_count(),
        };

        let fat_pg_first = bpb.fat_pg_first();
        let fat_pg_count = bpb.fat_pg_count();

        let mut fat = Fat::alloc(
            fat_type,
            device_dim.page_size,
            fat_pg_first,
            fat_pg_count,
            geometry.data_cluster_count,
        );

        device.read_pages(fat_pg_first, fat.as_raw_mut()).await?;
        fat.recompute_metadata();

        let fat = Arc::new(AsyncMutex::new(fat));

        let root = FatDirectory::open_root(geometry, device.clone(), fat.clone()).await?;

        Ok(Self {
            device,
            geometry,
            fat,
            root,
        })
    }

    pub fn root(&self) -> Arc<dyn Directory> {
        self.root.clone() as Arc<dyn Directory>
    }

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
                32 + fat_count * fat_pg_count + cluster_pg_count
            } else {
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
        if fat32 {
            fat.make_eoc(0);
        }
        fat.flush(device.clone()).await?;

        let geometry = FatGeometry {
            fat_type,
            data_cluster_count,
            cluster_pg_count,
            data_region_pg_first,
            root_cluster,
            root_dir_pg_first,
            root_entry_count,
        };
        let fat = Arc::new(AsyncMutex::new(fat));

        let root = FatDirectory::open_root(geometry, device.clone(), fat.clone()).await?;

        Ok(Self {
            device,
            geometry,
            fat,
            root,
        })
    }

    pub async fn flush(&self) -> IoResult<()> {
        self.fat.lock().await.flush(self.device.clone()).await?;
        self.device.flush().await
    }
}

impl Filesystem for FatVolume {
    fn root(&self) -> Arc<dyn Directory> {
        self.root.clone() as Arc<dyn Directory>
    }
}
