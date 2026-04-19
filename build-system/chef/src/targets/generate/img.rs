mod disk;
mod part;

use {
    crate::{
        args::{BuildProfile, GeneratePartition},
        config::CONFIG,
        paths,
        targets::{
            self,
            generate::img::{disk::DiskImgPartition, part::GenerateFatImgTarget},
        },
    },
    std::{path::PathBuf, rc::Rc},
};

pub use disk::GenerateDiskImgTarget;
use gpt::guid::Guid;

pub fn disk(profile: BuildProfile, page_size: usize, frame_size: usize) -> GenerateDiskImgTarget {
    GenerateDiskImgTarget::new(
        profile,
        page_size,
        frame_size,
        CONFIG.img_disk_size_mb,
        vec![
            DiskImgPartition::new(
                Guid::EFI_SYSTEM,
                "BOOT",
                CONFIG.img_part_boot_size_mb,
                PathBuf::from(paths::flat_dir(GeneratePartition::Boot, profile)),
                Rc::new(targets::generate::flat(GeneratePartition::Boot, profile)),
            ),
            DiskImgPartition::new(
                Guid::PENTOS_SYSTEM,
                "PENTOS",
                CONFIG.img_part_system_size_mb,
                PathBuf::from(paths::flat_dir(GeneratePartition::System, profile)),
                Rc::new(targets::generate::flat(GeneratePartition::System, profile)),
            ),
        ],
    )
}

pub fn boot(profile: BuildProfile, page_size: usize, frame_size: usize) -> GenerateFatImgTarget {
    GenerateFatImgTarget::into_file(
        PathBuf::from(paths::flat_dir(GeneratePartition::Boot, profile)),
        Rc::new(targets::generate::flat(GeneratePartition::Boot, profile)),
        paths::img(GeneratePartition::Boot, profile),
        CONFIG.img_part_boot_size_mb,
        page_size,
        frame_size,
    )
}

pub fn system(profile: BuildProfile, page_size: usize, frame_size: usize) -> GenerateFatImgTarget {
    GenerateFatImgTarget::into_file(
        PathBuf::from(paths::flat_dir(GeneratePartition::System, profile)),
        Rc::new(targets::generate::flat(GeneratePartition::System, profile)),
        paths::img(GeneratePartition::System, profile),
        CONFIG.img_part_system_size_mb,
        page_size,
        frame_size,
    )
}
