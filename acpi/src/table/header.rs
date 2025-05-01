use super::AcpiTable;
use core::slice;

#[repr(C, packed)]
pub struct AcpiHeader {
    pub sig: [u8; 4],
    pub len: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub oemtableid: [u8; 8],
    pub oemrevision: u32,
    pub creatorid: u32,
    pub creatorrevision: u32,
}

impl AcpiHeader {
    pub fn signature(&self) -> Option<&str> {
        Some(str::from_utf8(&self.sig).ok()?)
    }
    pub fn verify_checksum(&self) -> bool {
        let bytes = unsafe {
            // # Safety
            // Trust in the system vendor
            slice::from_raw_parts(self as *const _ as *const u8, self.len as usize)
        };
        let mut sum = 0u8;
        for &byte in bytes {
            sum = sum.wrapping_add(byte);
        }
        sum == 0
    }

    pub fn getas<T: AcpiTable>(&self) -> Option<&T> {
        if &self.sig == T::SIG && self.verify_checksum() {
            unsafe {
                // # Safety
                // If signature checks out, and checksum checks out,
                // then it's system vendor's fault if this is still unsafe, not mine
                Some(&*(self as *const _ as *const T))
            }
        } else {
            None
        }
    }
}
