#[repr(C)]
pub struct ShortDirEntry {
    name: [u8; 11],
    attributes: u8,
    nt_res: u8,
    creat_time_cs: u8, // units of centi-seconds... the kinda unit you use once in a lifetime
    creat_time: u16,
    creat_date: u16,

    last_acc_date: u16,

    cluster_hi: u16,

    write_time: u16,
    write_date: u16,

    cluster_lo: u16,

    file_size: u32,
}

#[repr(C, packed)]
pub struct LongFileNameEntry {
    order: u8,
    name0: [u16; 5],
    attributes: u8,
    typ: u8,
    checksum: u8,
    name1: [u16; 6],
    cluster_lo: u16,
    name2: [u16; 2],
}

impl ShortDirEntry {
    pub const fn cluster(&self) -> usize {
        self.cluster_lo as usize | (self.cluster_hi as usize) << 16
    }
}
