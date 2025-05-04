use super::AcpiHeader;
use super::AcpiTable;
use super::Signature;
use super::XSDT_SIG;
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
    pub const fn entry_count(&self) -> usize {
        const HEADER_SIZE: usize = mem::size_of::<AcpiHeader>();
        (self.header.len as usize - HEADER_SIZE) / 8
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

    pub fn find_unique<T: AcpiTable>(&self) -> &T {
        let Some(header) = self.entries().find(|entry| &entry.sig == T::SIG) else {
            if let Some(str_sig) = T::SIG.as_ascii() {
                panic!("ACPI table: {} not found", str_sig.as_str());
            } else {
                panic!("ACPI tabke: {:?} not found", T::SIG);
            }
        };
        header.getas().expect("Corrupt ACPI table")
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

impl AcpiTable for Xsdt {
    const SIG: Signature = XSDT_SIG;
}
