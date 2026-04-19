//! FAT32 directory implementation.
//!
//! A `FatDirectory` is built on top of an `Arc<File>` whose backend is a
//! [`FatFileBackend`] in general ot a [`FatRootBackend`] for FAT16/12 root directory.
//! We treat directories as files internally, simplifying
//! the implementation. All cluster-management is shared with regular files,
//! and the chunk cache automatically caches directory pages.

mod entry;
mod name;
mod root;

use {
    crate::{
        FatGeometry,
        dir::{name::UpperName, root::FatRootBackend},
        dirent::{self, ATTR_DIRECTORY, DIRENT_SIZE, ShortDirEntry},
        fat::{Fat, FatType},
        file::FatFileBackend,
    },
    alloc::{boxed::Box, collections::btree_map::BTreeMap, sync::Arc, vec::Vec},
    block::BlockDevice,
    fs::{
        dir::{DirEntry, DirFuture, Directory, EntryKind},
        file::File,
    },
    io::{IoError, IoResult},
    sync::AsyncMutex,
};

pub struct FatDirectory {
    geometry: FatGeometry,
    device: Arc<dyn BlockDevice>,
    fat: Arc<AsyncMutex<Fat>>,

    first_cluster: usize,
    is_root: bool,
    state: AsyncMutex<DirState>,
}

struct DirState {
    storage: Arc<File>,
    file_cache: BTreeMap<UpperName, alloc::sync::Weak<File>>,
    dir_cache: BTreeMap<UpperName, alloc::sync::Weak<FatDirectory>>,
}

impl FatDirectory {
    /// Open an existing directory. `first_cluster` is **0-indexed**
    pub async fn open(
        geometry: FatGeometry,
        device: Arc<dyn BlockDevice>,
        fat: Arc<AsyncMutex<Fat>>,
        first_cluster: usize,
    ) -> IoResult<Arc<Self>> {
        Self::open_normal(geometry, device, fat, first_cluster, false).await
    }

    /// Open the volume's root directory.
    /// Depending on FAT type, the root is either like any other directory,
    /// or it is special.
    pub async fn open_root(
        geometry: FatGeometry,
        device: Arc<dyn BlockDevice>,
        fat: Arc<AsyncMutex<Fat>>,
    ) -> IoResult<Arc<Self>> {
        match geometry.fat_type {
            FatType::Fat32 => {
                let root_0based = geometry.root_cluster - 2;
                Self::open_normal(geometry, device, fat, root_0based, true).await
            }
            FatType::Fat16 => Self::open_fat1612_root(geometry, device, fat).await,
            FatType::Fat12 => Err(IoError::Unsupported),
        }
    }

    /// Use `FatRootBackend` in a `File` and build the directory around it.
    async fn open_fat1612_root(
        geometry: FatGeometry,
        device: Arc<dyn BlockDevice>,
        fat: Arc<AsyncMutex<Fat>>,
    ) -> IoResult<Arc<Self>> {
        let page_size = device.dimensions().page_size;
        let region_pg_count = (geometry.root_entry_count * DIRENT_SIZE).div_ceil(page_size);

        let backend =
            FatRootBackend::new(device.clone(), geometry.root_dir_pg_first, region_pg_count);
        let storage = File::from_backend(Box::new(backend));

        Ok(Arc::new(Self {
            geometry,
            device,
            fat,
            first_cluster: 0,
            is_root: true,
            state: AsyncMutex::new(DirState {
                storage,
                file_cache: BTreeMap::new(),
                dir_cache: BTreeMap::new(),
            }),
        }))
    }

    async fn open_normal(
        geometry: FatGeometry,
        device: Arc<dyn BlockDevice>,
        fat: Arc<AsyncMutex<Fat>>,
        first_cluster: usize,
        is_root: bool,
    ) -> IoResult<Arc<Self>> {
        // Open the backend once with size 0 just to count clusters,
        // then rebuild with the real size. I can't bother with finding a way to
        // not run the clusters once
        let probe = FatFileBackend::open(
            geometry,
            device.clone(),
            fat.clone(),
            Some(first_cluster),
            0,
        )
        .await?;
        let cluster_count = probe.cluster_count();
        drop(probe);

        let cluster_size = geometry.cluster_pg_count * device.dimensions().page_size;
        let size = cluster_count * cluster_size;

        let backend = FatFileBackend::open(
            geometry,
            device.clone(),
            fat.clone(),
            Some(first_cluster),
            size,
        )
        .await?;
        let storage = File::from_backend(Box::new(backend));

        Ok(Arc::new(Self {
            geometry,
            device,
            fat,
            first_cluster,
            is_root,
            state: AsyncMutex::new(DirState {
                storage,
                file_cache: BTreeMap::new(),
                dir_cache: BTreeMap::new(),
            }),
        }))
    }
}

impl FatDirectory {
    async fn list_impl(&self) -> IoResult<Vec<DirEntry>> {
        let state = self.state.lock().await;
        let entries = entry::scan_directory(&state.storage).await?;
        Ok(entries
            .into_iter()
            .map(|e| DirEntry {
                kind: if e.is_dir() {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                },
                name: e.display_name(),
            })
            .collect())
    }
}

impl Directory for FatDirectory {
    fn list<'a>(&'a self) -> DirFuture<'a, Vec<DirEntry>> {
        Box::pin(self.list_impl())
    }

    fn open_file<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<File>> {
        Box::pin(self.open_file_impl(name))
    }

    fn open_dir<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<dyn Directory>> {
        Box::pin(self.open_dir_impl(name))
    }

    fn create_file<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<File>> {
        Box::pin(self.create_file_impl(name))
    }

    fn create_dir<'a>(&'a self, name: &'a str) -> DirFuture<'a, Arc<dyn Directory>> {
        Box::pin(self.create_dir_impl(name))
    }
}

impl FatDirectory {
    async fn open_file_impl(&self, name: &str) -> IoResult<Arc<File>> {
        let key = UpperName::from_str(name);

        let mut state = self.state.lock().await;
        if let Some(weak) = state.file_cache.get(&key)
            && let Some(arc) = weak.upgrade()
        {
            return Ok(arc);
        }

        let entries = entry::scan_directory(&state.storage).await?;
        let resolved = entries
            .iter()
            .find(|e| UpperName::from_str(&e.display_name()) == key)
            .ok_or(IoError::NotFound)?;

        if resolved.is_dir() {
            return Err(IoError::NotFile);
        }

        let sfn_slot = resolved.slot_range.end - 1;

        let raw_cluster_nb = resolved.sfn.cluster();
        let first_cluster = if raw_cluster_nb < 2 {
            None
        } else {
            Some(raw_cluster_nb - 2)
        };

        let backend = FatFileBackend::open(
            self.geometry,
            self.device.clone(),
            self.fat.clone(),
            first_cluster,
            resolved.sfn.file_size as usize,
        )
        .await?
        .with_parent_entry(state.storage.clone(), sfn_slot);

        let file = File::from_backend(Box::new(backend));
        state.file_cache.insert(key, Arc::downgrade(&file));

        Ok(file)
    }

    async fn open_dir_impl(&self, name: &str) -> IoResult<Arc<dyn Directory>> {
        let key = UpperName::from_str(name);

        let mut state = self.state.lock().await;
        if let Some(weak) = state.dir_cache.get(&key)
            && let Some(arc) = weak.upgrade()
        {
            return Ok(arc);
        }

        let entries = entry::scan_directory(&state.storage).await?;
        let resolved = entries
            .iter()
            .find(|e| UpperName::from_str(&e.display_name()) == key)
            .ok_or(IoError::NotFound)?;
        if !resolved.is_dir() {
            return Err(IoError::NotDir);
        }

        let dir_cluster = resolved.sfn.cluster() - 2;
        let dir = FatDirectory::open(
            self.geometry,
            self.device.clone(),
            self.fat.clone(),
            dir_cluster,
        )
        .await?;

        state.dir_cache.insert(key, Arc::downgrade(&dir));

        Ok(dir)
    }

    async fn create_file_impl(&self, name: &str) -> IoResult<Arc<File>> {
        name::validate_name(name)?;
        let key = UpperName::from_str(name);

        let mut state = self.state.lock().await;

        let entries = entry::scan_directory(&state.storage).await?;
        if entries
            .iter()
            .any(|e| UpperName::from_str(&e.display_name()) == key)
        {
            return Err(IoError::AlreadyExists);
        }

        let sfn_name = entry::allocate_sfn(name, &entries);
        let checksum = dirent::sfn_checksum(&sfn_name);
        let lfns = dirent::build_lfn_entries(name, checksum);

        let total_slots = lfns.len() + 1;
        let start_slot = match entry::find_free_slots(&state.storage, total_slots).await? {
            Some(s) => s,
            None => {
                grow_dir(&state.storage).await?;
                entry::find_free_slots(&state.storage, total_slots)
                    .await?
                    .ok_or(IoError::NoSpace)?
            }
        };

        let mut sfn = ShortDirEntry {
            name: sfn_name,
            attributes: 0,
            nt_res: 0,
            creat_time_cs: 0,
            creat_time: 0,
            creat_date: 0,
            last_acc_date: 0,
            cluster_hi: 0,
            write_time: 0,
            write_date: 0,
            cluster_lo: 0,
            file_size: 0,
        };
        sfn.set_cluster(0);

        entry::write_entry_set(&state.storage, start_slot, &lfns, &sfn).await?;

        let sfn_slot = start_slot + lfns.len();
        let backend =
            FatFileBackend::new_empty(self.geometry, self.device.clone(), self.fat.clone())
                .with_parent_entry(state.storage.clone(), sfn_slot);
        let file = File::from_backend(Box::new(backend));

        state.file_cache.insert(key, Arc::downgrade(&file));
        Ok(file)
    }

    async fn create_dir_impl(&self, name: &str) -> IoResult<Arc<dyn Directory>> {
        name::validate_name(name)?;

        let key = UpperName::from_str(name);
        let mut state = self.state.lock().await;

        let entries = entry::scan_directory(&state.storage).await?;
        if entries
            .iter()
            .any(|e| UpperName::from_str(&e.display_name()) == key)
        {
            return Err(IoError::AlreadyExists);
        }

        let new_cluster = {
            let mut fat = self.fat.lock().await;
            fat.cluster_alloc().ok_or(IoError::NoSpace)?
        };

        let sfn_name = entry::allocate_sfn(name, &entries);

        let checksum = dirent::sfn_checksum(&sfn_name);

        let lfns = dirent::build_lfn_entries(name, checksum);
        let total_slots = lfns.len() + 1;
        let start_slot = match entry::find_free_slots(&state.storage, total_slots).await? {
            Some(s) => s,
            None => {
                grow_dir(&state.storage).await?;
                entry::find_free_slots(&state.storage, total_slots)
                    .await?
                    .ok_or(IoError::NoSpace)?
            }
        };

        let mut sfn = ShortDirEntry {
            name: sfn_name,
            attributes: ATTR_DIRECTORY,
            nt_res: 0,
            creat_time_cs: 0,
            creat_time: 0,
            creat_date: 0,
            last_acc_date: 0,
            cluster_hi: 0,
            write_time: 0,
            write_date: 0,
            cluster_lo: 0,
            file_size: 0,
        };
        sfn.set_cluster(new_cluster + 2);

        let parent_cluster = if self.is_root {
            None
        } else {
            Some(self.first_cluster)
        };

        entry::write_dot_entries(&self.geometry, &self.device, new_cluster, parent_cluster).await?;
        entry::write_entry_set(&state.storage, start_slot, &lfns, &sfn).await?;

        {
            let mut fat = self.fat.lock().await;
            fat.flush(self.device.clone()).await?;
        }

        let dir = FatDirectory::open(
            self.geometry,
            self.device.clone(),
            self.fat.clone(),
            new_cluster,
        )
        .await?;

        state.dir_cache.insert(key, Arc::downgrade(&dir));

        Ok(dir as Arc<dyn Directory>)
    }
}

/// Grow the directory by one cluster, zeroing the new slots
async fn grow_dir(storage: &Arc<File>) -> IoResult<()> {
    let cluster_size = storage.chunk_size();
    let new_size = storage.size() + cluster_size;

    let mut open = storage.open();

    // This will fail for FatRootBackend, which means that we reached the maximum number
    // of entries for the root directory, can't do much about it.
    open.resize(new_size).await?;

    let zeros = alloc::vec![0u8; cluster_size];
    open.seek(new_size - cluster_size);
    open.write_all(&zeros).await?;
    Ok(())
}
