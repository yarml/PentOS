use super::Xsdt;
use core::mem;
use core::slice;

#[repr(C, packed)]
pub struct Rsdp {
    pub sig: [u8; 8],
    pub legacy_checksum: u8,
    pub oemid: [u8; 6],
    pub revivion: u8,
    pub rsdt_address: u32,
    pub len: u32,
    pub xsdt_address: u64,
    pub checksum: u8,
    reserved: [u8; 3],
}

impl Rsdp {
    pub fn verify(&self) -> bool {
        if &self.sig != b"RSD PTR " || (self.len as usize) < mem::size_of::<Rsdp>() {
            return false;
        }
        let bytes =
            unsafe { slice::from_raw_parts(self as *const _ as *const u8, self.len as usize) };
        let mut sum = 0u8;
        for &byte in bytes {
            sum = sum.wrapping_add(byte);
        }
        sum == 0
    }

    pub fn xsdt(&self) -> &Xsdt {
        let xsdt = self.xsdt_address as *const Xsdt;
        unsafe { &*xsdt }
    }
}
