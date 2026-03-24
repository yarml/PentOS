use {crate::guid::Guid, alloc::string::String, block::BlockDeviceSize, core::ops::RangeInclusive};

const EFI_PART_SIGNATURE: u64 = 0x5452415020494645;
const EFI_PART_REVISION: u32 = 0x00010000;

const GPT_HEADER_MIN_SIZE: usize = 92;
const GPT_PART_ENTRY_SIZE: usize = core::mem::size_of::<PartitionEntry>();

#[repr(C, packed)]
pub struct GptHeader {
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
    partlist_lba: u64,
    partlist_cap: u32,
    part_entry_size: u32,
    partlist_crc32: u32,
    res1: [u8],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PartitionEntry {
    pub type_guid: Guid,
    pub guid: Guid,
    pub lba_start: u64,
    pub lba_end: u64,
    attributes: u64,
    name: [u16; 36],
}

#[derive(Default, Clone, Copy)]
pub struct FormatOptions {
    /// Anything below 128 will be ignored and 128 will be the actual capacity allocated
    pub partition_capacity: Option<u32>,
    /// If not specified, a Guid will be generated
    pub guid: Option<Guid>,

    pub full_zero: bool,
}

// Constructors
impl GptHeader {
    pub fn from_raw(sector: &[u8]) -> &Self {
        let sector_size = sector.len();
        let res1_size = sector_size - GPT_HEADER_MIN_SIZE;
        unsafe { &*core::ptr::from_raw_parts(sector.as_ptr(), res1_size) }
    }
    pub fn from_raw_mut(sector: &mut [u8]) -> &mut Self {
        let sector_size = sector.len();
        let res1_size = sector_size - GPT_HEADER_MIN_SIZE;
        unsafe { &mut *core::ptr::from_raw_parts_mut(sector.as_mut_ptr(), res1_size) }
    }

    pub fn as_raw(&self) -> &[u8] {
        let total_size = GPT_HEADER_MIN_SIZE + self.res1.len();
        unsafe { &*core::ptr::from_raw_parts(self as *const _ as *const u8, total_size) }
    }
    pub fn as_raw_mut(&mut self) -> &mut [u8] {
        let total_size = GPT_HEADER_MIN_SIZE + self.res1.len();
        unsafe { &mut *core::ptr::from_raw_parts_mut(self as *mut _ as *mut u8, total_size) }
    }
}

impl PartitionEntry {
    pub const NULL: PartitionEntry = PartitionEntry {
        attributes: 0,
        guid: Guid::NULL,
        type_guid: Guid::NULL,
        name: [0; 36],
        lba_start: 0,
        lba_end: 0,
    };

    pub fn from_raw(buf: &[u8], cap: usize) -> &[PartitionEntry] {
        assert!(buf.len() >= cap * GPT_PART_ENTRY_SIZE);
        unsafe { &*core::ptr::from_raw_parts(buf.as_ptr(), cap) }
    }
    pub fn from_raw_mut(buf: &mut [u8], cap: usize) -> &mut [PartitionEntry] {
        assert!(buf.len() >= cap * GPT_PART_ENTRY_SIZE);
        unsafe { &mut *core::ptr::from_raw_parts_mut(buf.as_mut_ptr(), cap) }
    }

    pub fn into_raw(partlist: &[PartitionEntry]) -> &[u8] {
        unsafe {
            &*core::ptr::from_raw_parts(partlist.as_ptr(), partlist.len() * GPT_PART_ENTRY_SIZE)
        }
    }
    pub fn into_raw_mut(partlist: &mut [PartitionEntry]) -> &mut [u8] {
        unsafe {
            &mut *core::ptr::from_raw_parts_mut(
                partlist.as_mut_ptr(),
                partlist.len() * GPT_PART_ENTRY_SIZE,
            )
        }
    }
}

// Simple accessors
impl GptHeader {
    pub fn alternate_lba(&self) -> u64 {
        self.alternate_lba
    }

    pub fn usable_lba(&self) -> RangeInclusive<u64> {
        self.first_usable_lba..=self.last_usable_lba
    }

    pub fn partlist_lba(&self, sector_size: usize) -> RangeInclusive<u64> {
        let partlist_size = 128 * self.partlist_cap as u64;
        let partlist_sector_count = partlist_size.div_ceil(sector_size as u64);
        self.partlist_lba..=(self.partlist_lba + partlist_sector_count - 1)
    }

    pub fn partlist_cap(&self) -> usize {
        self.partlist_cap as usize
    }

    pub fn disk_guid(&self) -> Guid {
        self.disk_guid
    }
}

impl FormatOptions {
    pub const fn with_guid(self, guid: Guid) -> Self {
        Self {
            guid: Some(guid),
            ..self
        }
    }
}

// Heavy duty procedures
impl GptHeader {
    pub fn format(&mut self, size: BlockDeviceSize, main: bool, options: FormatOptions) {
        let last_lba = size.sector_count - 1;

        let partition_cap = u32::max(options.partition_capacity.unwrap_or(128), 128);
        let partlist_size = 128 * partition_cap as usize;
        let partlist_sector_count = partlist_size.div_ceil(size.sector_size);

        self.res0 = 0;
        self.res1.fill(0);

        self.signature = EFI_PART_SIGNATURE;
        self.revision = EFI_PART_REVISION;
        self.header_size = (self.res1.len() + 92) as u32;

        self.my_lba = if main { 1 } else { last_lba };
        self.alternate_lba = if !main { 1 } else { last_lba };

        self.disk_guid = options.guid.unwrap_or_else(Guid::gen_v4);

        self.partlist_lba = if main {
            2
        } else {
            last_lba - partlist_sector_count as u64
        };
        self.partlist_cap = partition_cap;
        self.partlist_crc32 = crypto::crc32_zdata(partlist_size);
        self.part_entry_size = 128;

        self.first_usable_lba = partlist_sector_count as u64 + 2;
        self.last_usable_lba = last_lba - partlist_sector_count as u64 - 1;

        self.header_crc32 = 0;

        let self_raw = self.as_raw();
        self.header_crc32 = crypto::crc32(self_raw);
    }

    /// Performs the most basic check of the signature only
    pub fn sanity_check(&self) -> bool {
        self.signature == EFI_PART_SIGNATURE
    }

    pub fn check(
        p_header: &Self,
        b_header: &Self,
        p_partlist: &[PartitionEntry],
        b_partlist: &[PartitionEntry],
    ) -> bool {
        // Sanity check
        if p_header.signature != EFI_PART_SIGNATURE {
            return false;
        }
        // Basic information matching
        if p_header.my_lba != b_header.alternate_lba
            || b_header.my_lba != p_header.alternate_lba
            || p_header.partlist_crc32 != b_header.partlist_crc32
        {
            return false;
        }

        // Header CRC32
        if crypto::crc32(p_header.as_raw()) != 0 {
            return false;
        }
        if crypto::crc32(b_header.as_raw()) != 0 {
            return false;
        }

        // Partlist CRC32
        if crypto::crc32(PartitionEntry::into_raw(p_partlist)) != p_header.partlist_crc32 {
            return false;
        }
        if crypto::crc32(PartitionEntry::into_raw(b_partlist)) != b_header.partlist_crc32 {
            return false;
        }

        true
    }

    pub fn update_partlist_crc32(&mut self, new_partlist_crc32: u32) {
        self.partlist_crc32 = new_partlist_crc32;
        self.header_crc32 = 0;
        let new_crc32 = crypto::crc32(self.as_raw());
        self.header_crc32 = new_crc32;
    }
}

impl PartitionEntry {
    pub fn new(
        guid: Option<Guid>,
        type_guid: Guid,
        lba_start: u64,
        lba_end: u64,
        name: &str,
    ) -> Self {
        let guid = guid.unwrap_or_else(Guid::gen_v4);
        let name = Self::str_to_gpt_name(name).expect("invalid partition name");

        Self {
            type_guid,
            guid,
            lba_start,
            lba_end,
            attributes: 0,
            name,
        }
    }

    pub fn name(&self) -> String {
        Self::gpt_name_to_string(&self.name)
    }

    fn str_to_gpt_name(s: &str) -> Result<[u16; 36], &'static str> {
        let mut buf = [0u16; 36];
        for (i, c) in s.encode_utf16().enumerate() {
            if i >= 35 {
                return Err("partition name too long");
            }
            buf[i] = c;
        }
        Ok(buf)
    }

    fn gpt_name_to_string(buf: &[u16; 36]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(36);
        String::from_utf16_lossy(&buf[..end])
    }
}
