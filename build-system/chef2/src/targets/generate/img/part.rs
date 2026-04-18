use {
    crate::{
        fs::block::FileBlockDevice,
        result::ResultExt,
        status::Status,
        target::{
            Target,
            run_policy::{AlwaysRun, CombinedRunPolicies, FilesNotExist, MirrorDeps, RunPolicy},
        },
        task,
    },
    ::fs::Directory,
    block::BlockDevice,
    fat32::{FatVolume, FormatOptions, media::MediaType},
    io::IoResult,
    std::{
        fs,
        os::unix::fs::MetadataExt,
        path::{Path, PathBuf},
        rc::Rc,
        sync::Arc,
    },
};

pub struct GenerateFatImgTarget {
    source: PathBuf,
    source_target: Rc<dyn Target>,
    variant: GenerateFatImgTargetVariant,
}

enum GenerateFatImgTargetVariant {
    IntoDevice {
        device: Arc<dyn BlockDevice>,
    },
    IntoFile {
        path: PathBuf,
        size_mb: usize,
        page_size: usize,
        frame_size: usize,
    },
}

impl GenerateFatImgTarget {
    pub const fn into_device(
        source: PathBuf,
        source_target: Rc<dyn Target>,
        device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            source,
            source_target,
            variant: GenerateFatImgTargetVariant::IntoDevice { device },
        }
    }

    pub const fn into_file(
        source: PathBuf,
        source_target: Rc<dyn Target>,
        output: PathBuf,
        size_mb: usize,
        page_size: usize,
        frame_size: usize,
    ) -> Self {
        Self {
            source,
            source_target,
            variant: GenerateFatImgTargetVariant::IntoFile {
                path: output,
                size_mb,
                page_size,
                frame_size,
            },
        }
    }
}

impl Target for GenerateFatImgTarget {
    fn spec(&self) -> bool {
        Status::push("Generating", format!("partition from {:?}", self.source));
        let device = match &self.variant {
            GenerateFatImgTargetVariant::IntoDevice { device } => device.clone(),
            GenerateFatImgTargetVariant::IntoFile {
                path,
                size_mb,
                page_size,
                frame_size,
            } => {
                let size = size_mb * 1024 * 1024;
                let page_count = size / page_size;
                Arc::new(
                    if let Ok(existing_size) =
                        fs::metadata(path).map(|metadata| metadata.size() as usize)
                        && existing_size == size
                    {
                        Status::doing("Reusing", format!("partition file {path:?}"));
                        FileBlockDevice::open(path, *page_size, *frame_size)
                    } else {
                        Status::doing("Creating", format!("partition file {path:?}"));
                        FileBlockDevice::create(path, *page_size, page_count, *frame_size)
                    },
                )
            }
        };

        Status::doing("Formatting", "FAT volume");

        let makeimg = async || {
            let volume = FatVolume::format(
                device.clone(),
                FormatOptions {
                    label: None,
                    id: None,
                    media: Some(MediaType::Removable),
                    full_zero: false,
                },
            )
            .await
            .or_fatal("FAT format");

            let root_dir = volume.root();

            Status::doing(
                "Populating",
                format!("FAT volume from {source:?}", source = self.source),
            );
            populate(&root_dir, &self.source)
                .await
                .or_fatal("populate FAT filesystem");
            volume.flush().await.or_fatal("FAT flush");
        };

        task::block_on(makeimg());

        Status::pop();
        true
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        match &self.variant {
            GenerateFatImgTargetVariant::IntoDevice { .. } => Box::new(AlwaysRun),
            GenerateFatImgTargetVariant::IntoFile { path, .. } => {
                Box::new(CombinedRunPolicies(vec![
                    Box::new(MirrorDeps),
                    Box::new(FilesNotExist::one_file(path.clone())),
                ]))
            }
        }
    }

    fn dependencies(&self) -> Vec<Rc<dyn Target>> {
        vec![self.source_target.clone()]
    }
}

/// Recursively copy the contents of `host_dir` into `fat_dir`.
///
/// Every file in `host_dir` becomes a file in `fat_dir`; every
/// subdirectory becomes a subdirectory, populated recursively.
/// Symlinks are followed (std::fs::read_dir follows them by default).
///
/// Hidden files (names starting with `.`) are included. The function
/// does NOT skip `.` or `..` because `std::fs::read_dir` never yields
/// them.
///
/// # Errors
///
/// Returns the first I/O error encountered, whether from the host
/// filesystem or from the FAT layer. The FAT volume may be partially
/// populated on error.
pub async fn populate(fat_dir: &Arc<dyn Directory>, host_dir: &Path) -> IoResult<()> {
    let entries = std::fs::read_dir(host_dir).map_err(|_| io::IoError::NotFound)?;

    for entry in entries {
        let entry = entry.map_err(|_| io::IoError::Unknown)?;
        let file_type = entry.file_type().map_err(|_| io::IoError::Unknown)?;
        let name = entry.file_name();
        let name_str = name.to_str().ok_or(io::IoError::InvalidInput)?;
        let full_path = entry.path();

        if file_type.is_dir() {
            let sub = fat_dir.create_dir(name_str).await?;
            // Recurse into the subdirectory.
            Box::pin(populate(&sub, &full_path)).await?;
        } else if file_type.is_file() {
            let file = fat_dir.create_file(name_str).await?;
            let host_bytes = std::fs::read(&full_path).map_err(|_| io::IoError::Unknown)?;

            if !host_bytes.is_empty() {
                let mut cursor = file.open();
                cursor.resize(host_bytes.len()).await?;
                cursor.write_all(&host_bytes).await?;
            }

            file.flush().await?;
        }
        // Symlinks that resolve to files/dirs are already handled above
        // because `file_type()` follows symlinks. Other special file
        // types (sockets, pipes) are silently skipped — they have no
        // FAT equivalent.
    }

    Ok(())
}
