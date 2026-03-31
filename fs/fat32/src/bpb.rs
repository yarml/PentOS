use {
    crate::{fat::FatType, format::disk_table, media::MediaType},
    block::BlockDeviceDimensions,
};

const OEM_NAME: [u8; 8] = *b"PENT FAT";
const VERSION: u16 = 0;
const BOOT_SIG: u8 = 0x29;

const FAT32_TYPE: [u8; 8] = *b"FAT32   ";
const FAT16_TYPE: [u8; 8] = *b"FAT16   ";
// Uncomment if we want to support formatting to FAT12
// const FAT12_TYPE: [u8; 8] = *b"FAT12   ";

const BPB_SIG: u16 = 0xAA55;
const BPB_MIN_SIZE: usize = 512;

const FAT_DIRENTRY_SIZE: usize = 32;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    jmp_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_page: u16,
    pages_per_cluster: u8,
    reserved_pages_count: u16,
    fat_count: u8,
    root_entry_count: u16,
    page_count_16: u16,
    media: u8,
    fat_size_16: u16,
    pages_per_track: u16,
    head_count: u16,
    hidden_pages: u32,
    page_count_32: u32,
    ext: BiosParameterBlockExt,
    bpb_sig: u16,
    res0: [u8], // likely size 0
}

#[repr(C, packed)]
union BiosParameterBlockExt {
    fat12_16: BiosParameterBlockExt12_16,
    fat32: BiosParameterBlockExt32,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct BiosParameterBlockExt32 {
    fat_size_32: u32,
    ext_flags: u16,
    version: u16,
    root_cluster: u32,
    info_pg: u16,
    b_bpb_pg: u16,
    res0: [u8; 12],
    drive_nb: u8,
    res1: u8,
    boot_sig: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fstype: [u8; 8],
    res2: [u8; 420],
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct BiosParameterBlockExt12_16 {
    drive_nb: u8,
    res0: u8,
    boot_sig: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fstype: [u8; 8],
    res1: [u8; 448],
}

impl BiosParameterBlock {
    pub fn from_raw_mut(page: &mut [u8]) -> &mut Self {
        let res0_size = page.len() - BPB_MIN_SIZE;
        unsafe { &mut *core::ptr::from_raw_parts_mut(page.as_mut_ptr(), res0_size) }
    }
}

impl BiosParameterBlock {
    pub const fn root_dir_pg_count(&self) -> usize {
        (self.root_entry_count as usize * FAT_DIRENTRY_SIZE).div_ceil(self.bytes_per_page as usize)
    }
    pub const fn root_entry_count(&self) -> usize {
        if matches!(self.fat_type(), FatType::Fat32) {
            0
        } else {
            self.root_entry_count as usize
        }
    }
    pub const fn fat_pg_first(&self) -> usize {
        if matches!(self.fat_type(), FatType::Fat32) {
            32
        } else {
            1
        }
    }
    pub const fn fat_pg_count(&self) -> usize {
        if self.fat_size_16 != 0 {
            self.fat_size_16 as usize
        } else {
            unsafe { self.ext.fat32.fat_size_32 as usize }
        }
    }
    pub const fn fat_count(&self) -> usize {
        self.fat_count as usize
    }
    pub const fn cluster_pg_count(&self) -> usize {
        self.pages_per_cluster as usize
    }

    pub const fn total_page_count(&self) -> usize {
        if self.page_count_16 != 0 {
            self.page_count_16 as usize
        } else {
            self.page_count_32 as usize
        }
    }

    pub const fn data_region_pg_first(&self) -> usize {
        self.reserved_pages_count as usize
            + self.fat_count as usize * self.fat_pg_count()
            + self.root_dir_pg_count()
    }
    pub const fn data_region_page_count(&self) -> usize {
        let total = self.total_page_count();
        total
            - self.reserved_pages_count as usize
            - self.fat_count as usize * self.fat_pg_count()
            - self.root_dir_pg_count()
    }

    pub const fn root_cluster(&self) -> usize {
        if matches!(self.fat_type(), FatType::Fat32) {
            unsafe { self.ext.fat32.root_cluster as usize }
        } else {
            0
        }
    }

    pub const fn data_cluster_count(&self) -> usize {
        self.data_region_page_count() / self.pages_per_cluster as usize
    }
    pub const fn cluster_pg_first(&self, n: usize) -> usize {
        ((n - 2) * self.pages_per_cluster as usize) + self.data_region_pg_first()
    }

    pub const fn cluster_fat_offset(&self, n: usize) -> usize {
        match self.fat_type() {
            FatType::Fat12 => n + (n / 2), // God forgive whoever designed FAT12
            FatType::Fat16 => n * 2,
            FatType::Fat32 => n * 4,
        }
    }
    pub const fn cluster_fat_pg(&self, n: usize) -> usize {
        self.reserved_pages_count as usize
            + self.cluster_fat_offset(n) / self.bytes_per_page as usize
    }
    pub const fn cluster_fat_entry_offset(&self, n: usize) -> usize {
        self.cluster_fat_offset(n) % self.bytes_per_page as usize
    }

    pub const fn fat_type(&self) -> FatType {
        let cluster_count = self.data_cluster_count();
        if cluster_count < 4085 {
            FatType::Fat12
        } else if cluster_count < 65525 {
            FatType::Fat16
        } else {
            FatType::Fat32
        }
    }

    pub const fn root_dir_pg_first(&self) -> usize {
        match self.fat_type() {
            FatType::Fat12 | FatType::Fat16 => {
                self.data_region_pg_first() - self.root_dir_pg_count()
            }
            FatType::Fat32 => {
                self.cluster_pg_first(unsafe { self.ext.fat32.root_cluster as usize })
            }
        }
    }
}

// Heavy duty stuff
impl BiosParameterBlock {
    pub fn format(
        &mut self,
        device_dimensions: BlockDeviceDimensions,
        id: u32,
        label: [u8; 11],
        media: MediaType,
    ) {
        assert_eq!(core::mem::size_of::<BiosParameterBlockExt12_16>(), 474);
        assert_eq!(core::mem::size_of::<BiosParameterBlockExt32>(), 474);
        assert_eq!(core::mem::size_of::<BiosParameterBlockExt>(), 474);
        assert_eq!(core::mem::offset_of!(BiosParameterBlock, ext), 36);
        assert_eq!(core::mem::offset_of!(BiosParameterBlock, bpb_sig), 510);

        self.jmp_boot = [0xEB, 0xFE, 0x90]; // jmp self
        self.oem_name = OEM_NAME;
        self.bytes_per_page = device_dimensions.page_size as u16;
        self.fat_count = 2;
        self.media = media.code();
        self.pages_per_track = 0x3F; // I think we have nothing to care for this one
        self.head_count = 0xFF; // nor here
        self.hidden_pages = 0; // nor here
        self.bpb_sig = BPB_SIG;

        let fat32 = match disk_table::format_type(device_dimensions.page_count) {
            FatType::Fat12 => todo!("FAT12 formatting"),
            FatType::Fat16 => false,
            FatType::Fat32 => true,
        };

        if fat32 {
            self.ext.fat32.version = VERSION;
            self.ext.fat32.drive_nb = 0; // don't care
            self.ext.fat32.boot_sig = BOOT_SIG;
            self.ext.fat32.volume_id = id;
            self.ext.fat32.volume_label = label;
            self.ext.fat32.fstype = FAT32_TYPE;

            // FAT 32 exclusives
            self.ext.fat32.ext_flags = 0; // Mirroring active
            self.ext.fat32.info_pg = 1;
            self.ext.fat32.b_bpb_pg = 6;
            self.ext.fat32.root_cluster = 2;

            unsafe { self.ext.fat32.res0.fill(0) };
            self.ext.fat32.res1 = 0;
            unsafe { self.ext.fat32.res2.fill(0) };
        } else {
            self.ext.fat12_16.drive_nb = 0; // Don't care
            self.ext.fat12_16.boot_sig = BOOT_SIG;
            self.ext.fat12_16.volume_id = id;
            self.ext.fat12_16.volume_label = label;
            self.ext.fat12_16.fstype = FAT16_TYPE;

            self.ext.fat12_16.res0 = 0;
            unsafe { self.ext.fat12_16.res1.fill(0) };
        }

        if fat32 {
            self.reserved_pages_count = 32;
            self.root_entry_count = 0;
            self.page_count_16 = 0;
            self.page_count_32 = device_dimensions.page_count as u32;
            self.pages_per_cluster = disk_table::fat32(device_dimensions.page_count)
                .expect("incompatible disksize and partition type")
                as u8;
        } else {
            self.reserved_pages_count = 1;
            self.root_entry_count = 512;

            if device_dimensions.page_count <= u16::MAX as usize {
                self.page_count_16 = device_dimensions.page_count as u16;
                self.page_count_32 = 0;
            } else {
                self.page_count_16 = 0;
                self.page_count_32 = device_dimensions.page_count as u32;
            }

            self.pages_per_cluster = disk_table::fat16(device_dimensions.page_count)
                .expect("incompatible disksize and partition type")
                as u8;
        }

        {
            // Magic formulas from Microsoft to compute the FAT size
            let root_dir_page_count =
                (self.root_entry_count as usize * 32).div_ceil(self.bytes_per_page as usize);
            let t1 = device_dimensions.page_count
                - (self.reserved_pages_count as usize + root_dir_page_count);
            let t2 = (256 * self.pages_per_cluster as usize + self.fat_count as usize)
                / if fat32 { 2 } else { 1 };
            let fat_size = t1.div_ceil(t2);
            if fat32 {
                self.fat_size_16 = 0;
                self.ext.fat32.fat_size_32 = fat_size as u32;
            } else {
                self.fat_size_16 = (fat_size & 0xFFFF) as u16;
            }
        }
    }
}
