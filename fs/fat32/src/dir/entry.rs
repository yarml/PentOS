use core::ops::Range;

use alloc::vec::Vec;

use {block::BlockDevice, io::IoError};

use {
    alloc::{string::String, sync::Arc},
    fs::File,
    io::IoResult,
};

use crate::{
    FatGeometry,
    dirent::{self, ATTR_DIRECTORY, DIRENT_SIZE, LongFileNameEntry, ShortDirEntry, SlotKind},
};

pub struct ResolvedEntry {
    pub long_name: Option<String>,
    pub sfn: ShortDirEntry,
    pub slot_range: Range<usize>,
}

impl ResolvedEntry {
    pub fn display_name(&self) -> String {
        match &self.long_name {
            Some(s) => s.clone(),
            None => dirent::decode_sfn(&self.sfn.name),
        }
    }

    pub fn is_dir(&self) -> bool {
        self.sfn.is_dir()
    }
}

pub async fn write_entry_set(
    storage: &Arc<File>,
    start_slot: usize,
    lfns: &[LongFileNameEntry],
    sfn: &ShortDirEntry,
) -> IoResult<()> {
    let mut open = storage.open();
    open.seek(start_slot * DIRENT_SIZE);

    let mut buf = [0u8; DIRENT_SIZE];
    for lfn in lfns {
        buf.fill(0);
        lfn.write_to(&mut buf);

        open.write_all(&buf).await?;
    }

    buf.fill(0);
    sfn.write_to(&mut buf);

    open.seek((start_slot + lfns.len()) * DIRENT_SIZE);
    open.write_all(&buf).await?;

    storage.flush().await
}

pub fn allocate_sfn(long: &str, existing: &[ResolvedEntry]) -> [u8; 11] {
    let (basis, lossy) = dirent::basis_name(long);

    if !lossy {
        let collides = existing.iter().any(|e| e.sfn.name == basis);
        if !collides {
            return basis;
        }
    }

    // Append a number until unique.
    for n in 1..=999_999 {
        let mut candidate = basis;
        dirent::apply_numeric_tail(&mut candidate, n);

        if !existing.iter().any(|e| e.sfn.name == candidate) {
            return candidate;
        }
    }
    // if this happens, we will have a collision, but idc
    let mut candidate = basis;
    dirent::apply_numeric_tail(&mut candidate, 999_999);

    candidate
}

pub async fn find_free_slots(storage: &Arc<File>, count: usize) -> IoResult<Option<usize>> {
    let mut open = storage.open();

    let total_slots = storage.size() / DIRENT_SIZE;

    let mut run_start: Option<usize> = None;
    let mut run_len = 0usize;

    for slot_index in 0..total_slots {
        let mut buf = [0u8; DIRENT_SIZE];
        open.seek(slot_index * DIRENT_SIZE);
        match open.read_all(&mut buf).await {
            Ok(()) => {}
            Err(IoError::Eof) => break,
            Err(e) => return Err(e),
        }
        match dirent::classify(&buf) {
            SlotKind::Free => {
                if run_start.is_none() {
                    run_start = Some(slot_index);
                }
                run_len += 1;
                if run_len == count {
                    return Ok(run_start);
                }
            }
            SlotKind::End => {
                if run_start.is_none() {
                    run_start = Some(slot_index);
                    run_len = 0;
                }
                let trailing = total_slots - slot_index;
                if run_len + trailing >= count {
                    return Ok(run_start);
                } else {
                    return Ok(None);
                }
            }
            SlotKind::Lfn | SlotKind::Sfn => {
                run_start = None;
                run_len = 0;
            }
        }
    }
    Ok(None)
}

pub async fn write_dot_entries(
    geometry: &FatGeometry,
    device: &Arc<dyn BlockDevice>,
    dot_cluster: usize,
    dotdot_cluster: Option<usize>,
) -> IoResult<()> {
    let cluster_size = geometry.cluster_pg_count * device.dimensions().page_size;
    let mut buf = alloc::vec![0u8; cluster_size];

    let mut dot = ShortDirEntry {
        name: *b".          ",
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
    dot.set_cluster(dot_cluster + 2);

    let mut dotdot = ShortDirEntry {
        name: *b"..         ",
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
    dotdot.set_cluster(dotdot_cluster.map(|i| i + 2).unwrap_or(0));

    let dot_bytes: &mut [u8; DIRENT_SIZE] = (&mut buf[0..DIRENT_SIZE]).try_into().unwrap();
    dot.write_to(dot_bytes);
    let dotdot_bytes: &mut [u8; DIRENT_SIZE] =
        (&mut buf[DIRENT_SIZE..2 * DIRENT_SIZE]).try_into().unwrap();
    dotdot.write_to(dotdot_bytes);

    let pg = geometry.data_region_pg_first + geometry.cluster_pg_count * dot_cluster;
    device.write_pages(pg, &buf).await
}

pub async fn scan_directory(storage: &Arc<File>) -> IoResult<Vec<ResolvedEntry>> {
    let mut open = storage.open();

    let mut entries: Vec<ResolvedEntry> = Vec::new();

    let mut pending_lfn: Vec<LongFileNameEntry> = Vec::new();
    let mut pending_lfn_first: usize = 0;

    let total_slots = storage.size() / DIRENT_SIZE;

    for slot_index in 0..total_slots {
        let mut buf = [0u8; DIRENT_SIZE];

        open.seek(slot_index * DIRENT_SIZE);

        match open.read_all(&mut buf).await {
            Ok(()) => {}
            // EOF in the middle shouldn't happen but treat as end.
            Err(IoError::Eof) => break,
            Err(e) => return Err(e),
        }

        match dirent::classify(&buf) {
            SlotKind::End => break,
            SlotKind::Free => {
                // Inconsistent FS. We simply ignore the previous LFNs
                pending_lfn.clear();
            }
            SlotKind::Lfn => {
                let lfn = LongFileNameEntry::from_bytes(&buf);
                if pending_lfn.is_empty() {
                    pending_lfn_first = slot_index;
                }
                pending_lfn.push(lfn);
            }
            SlotKind::Sfn => {
                let sfn = ShortDirEntry::from_bytes(&buf);

                let is_dot = sfn.name[0] == b'.';
                if is_dot || sfn.is_volume_id() {
                    pending_lfn.clear();
                    continue;
                }

                // Validate any pending LFN run against the SFN.
                let long_name = if !pending_lfn.is_empty() {
                    let checksum = dirent::sfn_checksum(&sfn.name);

                    let checksum_good = pending_lfn.iter().all(|e| e.checksum == checksum);

                    if checksum_good {
                        Some(dirent::decode_lfn_set(&pending_lfn))
                    } else {
                        // If checksum no good, ignore LFNs.
                        None
                    }
                } else {
                    None
                };

                let start = if long_name.is_some() {
                    pending_lfn_first
                } else {
                    slot_index
                };

                pending_lfn.clear();

                entries.push(ResolvedEntry {
                    long_name,
                    sfn,
                    slot_range: start..(slot_index + 1),
                });
            }
        }
    }
    Ok(entries)
}
