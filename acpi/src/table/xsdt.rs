use super::AcpiHeader;
use core::mem;

#[repr(C, packed)]
pub struct Xsdt {
    pub header: AcpiHeader,
}

pub struct XsdtIter<'a> {
    xsdt: &'a Xsdt,
    index: usize,
}

impl Xsdt {
    pub fn verify(&self) -> bool {
        if &self.header.sig != b"XSDT" || (self.header.len as usize) < mem::size_of::<Xsdt>() {
            return false;
        }
        self.header.verify_checksum()
    }

    pub const fn entry_count(&self) -> usize {
        const HEADER_SIZE: usize = mem::size_of::<AcpiHeader>();
        let len = (self.header.len as usize - HEADER_SIZE) / 8;
        len
    }

    pub const fn entry_at(&self, index: usize) -> Option<&AcpiHeader> {
        if index >= self.entry_count() {
            return None;
        }
        const HEADER_SIZE: usize = mem::size_of::<AcpiHeader>();
        const ENTRY_SIZE: usize = mem::size_of::<u64>();
        unsafe {
            let ptr = (self as *const _ as *const u8)
                .add(HEADER_SIZE)
                .add(index * ENTRY_SIZE) as *const *const AcpiHeader;
            Some(&**ptr)
        }
    }

    pub const fn entries(&self) -> XsdtIter {
        XsdtIter {
            xsdt: self,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a Xsdt {
    type Item = &'a AcpiHeader;
    type IntoIter = XsdtIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries()
    }
}

impl<'a> Iterator for XsdtIter<'a> {
    type Item = &'a AcpiHeader;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.xsdt.entry_at(self.index);
        self.index += 1;
        entry
    }
}
