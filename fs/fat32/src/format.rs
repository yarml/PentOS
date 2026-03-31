/// Mimicking Microsoft's magic numbers and applying 0 brain energy into
/// understanding this thing
pub mod disk_table {
    use crate::fat::FatType;

    const FAT12_CUTOFF_PGC: usize = (4 * 1024 * 1024) / 512;
    const FAT16_CUTOFF_PGC: usize = (512 * 1024 * 1024) / 512;

    pub const fn format_type(page_count: usize) -> FatType {
        match page_count {
            ..FAT12_CUTOFF_PGC => FatType::Fat12,
            FAT12_CUTOFF_PGC..FAT16_CUTOFF_PGC => FatType::Fat16,
            FAT16_CUTOFF_PGC.. => FatType::Fat32,
        }
    }

    /**
    This is the table for FAT16 drives. As copied & adapted from Microsoft FAT spec.

    # Requirements
    For this table to work properly BPB_RsvdSecCnt must be 1, BPB_NumFATs
    must be 2, and BPB_RootEntCnt must be 512.

    # Note
    This table includes entries for disk sizes larger than 512 MiB
    even though typically only the entries for disks < 512 MiB in
    size are used.
    */
    pub const fn fat16(total_sectors: usize) -> Option<usize> {
        Some(match total_sectors {
            ..8400 => return None,
            8400..32680 => 2,
            32680..262144 => 4,
            262144..524288 => 8,
            524288..1048576 => 16,
            1048576..2097152 => 32,
            2097152..4194304 => 64,
            _ => return None,
        })
    }

    /**
    This is the table for FAT32 drives. As copied & adapted from Microsoft FAT spec.

    # Requirements
    For this table to work properly BPB_RsvdSecCnt must be 32, and BPB_NumFATs must be 2.

    # Note
    This table includes entries for disk
    sizes smaller than 512 MB even though typically only the entries
    for disks >= 512 MB in size are used.
    */
    pub const fn fat32(total_sectors: usize) -> Option<usize> {
        Some(match total_sectors {
            ..66600 => return None,
            66600..532480 => 1,
            532480..16777216 => 8,
            16777216..33554432 => 16,
            33554432..67108864 => 32,
            67108864.. => 64,
        })
    }
}
