const LEAD_SIG: u32 = 0x41615252;
const TRAIL_SIG: u32 = 0xAA550000;
const STRUC_SIG: u32 = 0x61417272;

const FSINFO_MIN_SIZE: usize = 512;

#[repr(C)]
pub struct FSInfo {
    lead_sig: u32,
    res0: [u8; 480],
    struc_sig: u32,
    free_count: u32,
    next_free: u32,
    res1: [u8; 12],
    trail_sig: u32,
    res2: [u8],
}

impl FSInfo {
    pub const fn from_raw_mut(page: &mut [u8]) -> &mut Self {
        let res2_size = page.len() - FSINFO_MIN_SIZE;
        unsafe { &mut *core::ptr::from_raw_parts_mut(page.as_mut_ptr(), res2_size) }
    }
}

impl FSInfo {
    pub fn format(&mut self) {
        self.lead_sig = LEAD_SIG;
        self.struc_sig = STRUC_SIG;
        self.trail_sig = TRAIL_SIG;

        self.free_count = 0xFFFFFFFF;
        self.next_free = 0xFFFFFFFF;

        self.res0.fill(0);
        self.res1.fill(0);
        self.res2.fill(0);
    }
}

impl FSInfo {
    pub const fn set_next_free(&mut self, next_free: Option<u32>) {
        self.next_free = next_free.unwrap_or(0xFFFFFFFF);
    }
    pub const fn set_free_count(&mut self, free_count: Option<u32>) {
        self.free_count = free_count.unwrap_or(0xFFFFFFFF);
    }
}
