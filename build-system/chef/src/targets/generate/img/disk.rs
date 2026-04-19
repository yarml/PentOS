use {
    crate::{
        args::{BuildProfile, GeneratePartition},
        fs::block::FileBlockDevice,
        paths,
        result::ResultExt,
        status::Status,
        target::{
            Target,
            run_policy::{AlwaysRun, RunPolicy},
        },
        targets::generate::img::part::GenerateFatImgTarget,
        task,
    },
    block::BlockDevice,
    gpt::{GptDisk, format::FormatOptions, guid::Guid},
    std::{fs, path::PathBuf, rc::Rc, sync::Arc},
};

pub struct DiskImgPartition {
    type_guid: Guid,
    name: String,
    size_mb: usize,
    source: PathBuf,
    source_target: Rc<dyn Target>,
}

pub struct GenerateDiskImgTarget {
    profile: BuildProfile,
    page_size: usize,
    frame_size: usize,

    size: usize,

    partitions: Vec<DiskImgPartition>,
}

impl GenerateDiskImgTarget {
    pub const fn new(
        profile: BuildProfile,
        page_size: usize,
        frame_size: usize,
        size_mb: usize,
        partitions: Vec<DiskImgPartition>,
    ) -> Self {
        Self {
            profile,
            page_size,
            frame_size,
            size: size_mb * 1024 * 1024,
            partitions,
        }
    }
}

impl DiskImgPartition {
    pub fn new(
        type_guid: Guid,
        name: &str,
        size_mb: usize,
        source: PathBuf,
        source_target: Rc<dyn Target>,
    ) -> Self {
        Self {
            type_guid,
            name: name.to_owned(),
            size_mb,
            source,
            source_target,
        }
    }
}

impl Target for GenerateDiskImgTarget {
    fn spec(&self) -> bool {
        let page_count = self.size / self.page_size;
        let img_path = paths::img(GeneratePartition::Disk, self.profile);

        Status::push("Generating", format!("disk image {img_path:?}"));
        let device = Arc::new(
            if let Ok(size) = fs::metadata(&img_path).map(|metadata| metadata.len() as usize)
                && size == self.size
            {
                Status::doing("Reusing", format!("disk file {img_path:?}"));
                FileBlockDevice::open(&img_path, self.page_size, self.frame_size)
            } else {
                Status::doing("Creating", format!("disk file {img_path:?}"));
                FileBlockDevice::create(&img_path, self.page_size, page_count, self.frame_size)
            },
        );

        Status::doing("Formatting", "GPT disk partition");
        let disk =
            task::block_on(GptDisk::format(device, FormatOptions::default())).or_fatal("format");

        let usable_pages = disk.usable_pages();

        let usable_space =
            (usable_pages.end() - usable_pages.start() + 1) as usize * self.page_size;
        let needed_space = self.partitions.iter().map(|p| p.size_mb).sum::<usize>() * 1024 * 1024;

        if usable_space < needed_space {
            Status::error("configured disk size cannot contain all partitions partitions");
        }

        let mut pg_start = *usable_pages.start();
        for partition in &self.partitions {
            let pg_count = (partition.size_mb * 1024 * 1024).div_ceil(self.page_size);

            let guid = task::block_on(disk.add_partition(
                partition.type_guid,
                &partition.name,
                pg_start,
                pg_start + pg_count - 1,
            ))
            .or_fatal("add partition");

            let part = Arc::new(task::block_on(disk.open_partition(guid)).unwrap());

            let target = GenerateFatImgTarget::into_device(
                partition.source.clone(),
                partition.source_target.clone(),
                part.clone(),
            );

            target.run();

            pg_start += pg_count;
        }

        task::block_on(disk.flush()).or_fatal("flush");

        Status::pop();

        true
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        Box::new(AlwaysRun)
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![]
    }
}
