const MBR_SIG: u16 = 0xAA55;

/// Minimum definition to represent Protective MBT in GPT
#[repr(C, packed)]
pub struct MasterBootRecord {
    boot_code: [u8; 440],
    disk_signature: u32,
    res0: u16,
    partitions: [MasterBootRecordPartition; 4],
    signature: u16,
    res1: [u8], // likely size 0
}

#[repr(C, packed)]
pub struct MasterBootRecordPartition {
    boot: u8,
    chs_starting: [u8; 3],
    os_type: u8,
    chs_ending: [u8; 3],
    pg_start: u32,
    pg_size: u32,
}

impl MasterBootRecord {
    pub fn from_raw_mut(page: &mut [u8]) -> &mut Self {
        let page_size = page.len();
        assert!(page_size >= 512);
        let res1_size = page_size - 512;
        unsafe { &mut *core::ptr::from_raw_parts_mut(page.as_mut_ptr(), res1_size) }
    }
    pub fn set_protective(&mut self, disk_page_count: u64) {
        self.boot_code.fill(0);
        self.disk_signature = 0;
        self.res0 = 0;
        self.partitions = [
            MasterBootRecordPartition::protective(disk_page_count),
            MasterBootRecordPartition::null(),
            MasterBootRecordPartition::null(),
            MasterBootRecordPartition::null(),
        ];
        self.signature = MBR_SIG;
        self.res1.fill(0);
    }
}

impl MasterBootRecordPartition {
    pub const fn protective(disk_page_count: u64) -> Self {
        Self {
            boot: 0x00,
            chs_starting: [0x00, 0x02, 0x00],
            os_type: 0xEE,
            chs_ending: [0xFF, 0xFF, 0xFF],
            pg_start: 1,
            pg_size: u64::min(disk_page_count - 1, u32::MAX as u64) as u32,
        }
    }

    pub const fn null() -> Self {
        Self {
            boot: 0,
            chs_starting: [0; 3],
            os_type: 0,
            chs_ending: [0; 3],
            pg_start: 0,
            pg_size: 0,
        }
    }
}
