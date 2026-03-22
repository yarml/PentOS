use {
    crate::{FormatOptions, guid::Guid},
    alloc::boxed::Box,
    block::BlockDeviceSize,
    block_cache::BlockCache,
    core::ops::{Range, RangeInclusive},
    io::IoResult,
};

const EFI_PART_SIGNATURE: u64 = 0x5452415020494645;
const EFI_PART_REVISION: u32 = 0x00010000;

#[repr(C, packed)]
pub struct GPTHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    header_crc32: u32,
    res0: u32,
    my_lba: u64,
    alternate_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    disk_guid: Guid,
    partition_list_lba: u64,
    partition_cap: u32,
    partition_entry_size: u32,
    partition_list_crc32: u32,
    res1: [u8],
}

impl GPTHeader {
    pub fn interpret_gpt_header(lba1: &mut [u8]) -> &mut Self {
        let sector_size = lba1.len();
        assert!(sector_size >= 512);
        let res1_size = sector_size - 92;
        unsafe { &mut *core::ptr::from_raw_parts_mut(lba1.as_mut_ptr(), res1_size) }
    }

    pub fn format(&mut self, size: BlockDeviceSize, main: bool, options: FormatOptions) {
        let last_lba = size.sector_count - 1;

        let partition_cap = u32::max(options.partition_capacity.unwrap_or(128), 128);
        let partition_list_size = 128 * partition_cap as usize;
        let partition_list_sector_count = partition_list_size.div_ceil(size.sector_size);

        self.res0 = 0;
        self.res1.fill(0);

        self.signature = EFI_PART_SIGNATURE;
        self.revision = EFI_PART_REVISION;
        self.header_size = (self.res1.len() + 92) as u32;

        self.my_lba = if main { 1 } else { last_lba };
        self.alternate_lba = if !main { 1 } else { last_lba };

        self.disk_guid = options.guid.unwrap_or_else(Guid::gen_v4);

        self.partition_list_lba = if main {
            2
        } else {
            last_lba - partition_list_sector_count as u64
        };
        self.partition_cap = partition_cap;
        self.partition_list_crc32 = crypto::crc32_zdata(partition_list_size);
        self.partition_entry_size = 128;

        self.first_usable_lba = partition_list_sector_count as u64 + 2;
        self.last_usable_lba = last_lba - partition_list_sector_count as u64 - 1;

        self.header_crc32 = 0;

        let self_raw = self.raw();
        self.header_crc32 = crypto::crc32(self_raw);
    }

    pub async fn check(&self, expected_lba: u64, block_cache: &BlockCache) -> IoResult<bool> {
        if self.signature != EFI_PART_SIGNATURE {
            return Ok(false);
        }
        if self.my_lba != expected_lba {
            return Ok(false);
        }
        {
            let self_raw = self.raw();
            if crypto::crc32(self_raw) != 0 {
                return Ok(false);
            }
        }
        {
            let mut crc = 0;
            let partition_list_sectors = self.partition_list_lba(block_cache.size().sector_size);
            for lba in partition_list_sectors {
                let lb_lock = block_cache.get_sector(lba).await?;
                let lb = lb_lock.lock().await;
                crc = crypto::crc32_continue(&lb, crc);
            }

            if crc != 0 {
                return Ok(false);
            }
        }
        if self.my_lba == 1 {
            let backup_header_lb_lock = block_cache.get_sector(self.alternate_lba).await?;
            let mut backup_header_lb = backup_header_lb_lock.lock().await;
            let backup_gpt_header = Self::interpret_gpt_header(&mut backup_header_lb);
            Box::pin(backup_gpt_header.check(self.alternate_lba, block_cache)).await
        } else {
            Ok(true)
        }
    }

    pub fn raw(&self) -> &[u8] {
        unsafe {
            &*core::ptr::slice_from_raw_parts(
                self as *const GPTHeader as *const u8,
                self.res1.len() + 92,
            )
        }
    }

    pub fn usable_lba(&self) -> RangeInclusive<u64> {
        self.first_usable_lba..=self.last_usable_lba
    }

    pub fn partition_list_lba(&self, sector_size: usize) -> Range<u64> {
        let partition_list_size = 128 * self.partition_cap as u64;
        let partition_list_sector_count = partition_list_size.div_ceil(sector_size as u64);
        self.partition_list_lba..(self.partition_list_lba + partition_list_sector_count)
    }
}
