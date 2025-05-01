use super::AcpiHeader;
use core::mem;

#[repr(C, packed)]
pub struct Madt {
    pub header: AcpiHeader,
    pub lapic_address: u32,
    pub flags: u32,
}

#[repr(C, packed)]
pub struct MadtEntryHeader {
    pub ty: u8,
    pub len: u8,
}

#[repr(C, packed)]
pub struct LocalApic {
    pub header: MadtEntryHeader,
    pub proc_uid: u8,
    pub apic_id: u8,
    pub flags: u32,
}

pub struct MadtIterator<'a> {
    madt: &'a Madt,
    cursor: usize,
}

impl Madt {
    pub fn verify(&self) -> bool {
        if &self.header.sig != b"APIC" || (self.header.len as usize) < mem::size_of::<Self>() {
            return false;
        }
        self.header.verify_checksum()
    }

    pub fn iter(&self) -> MadtIterator {
        MadtIterator {
            madt: self,
            cursor: mem::size_of::<Madt>(),
        }
    }
}

impl<'a> IntoIterator for &'a Madt {
    type Item = &'a MadtEntryHeader;
    type IntoIter = MadtIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> Iterator for MadtIterator<'a> {
    type Item = &'a MadtEntryHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.madt.header.len as usize {
            return None;
        }
        let entry = unsafe {
            &*((self.madt as *const _ as *const u8).add(self.cursor) as *const MadtEntryHeader)
        };
        self.cursor += entry.len as usize;
        Some(entry)
    }
}

impl MadtEntryHeader {
    pub fn type_as_str(ty: u8) -> Option<&'static str> {
        match ty {
            0 => Some("Processor Local APIC"),
            1 => Some("IO APIC"),
            2 => Some("Interrupt Source Override"),
            3 => Some("NMI Source"),
            4 => Some("Local APIC NMI"),
            5 => Some("Local APIC Address Override"),
            6 => Some("IO SAPIC"),
            7 => Some("Local SAPIC"),
            8 => Some("Platform Interrupt Sources"),
            9 => Some("Local X2APIC"),
            10 => Some("Local X2APIC NMI"),
            11 => Some("Local X2APIC Address Override"),
            12 => Some("GIC"),
            13 => Some("GIC Distributor"),
            14 => Some("GIC Redistributor"),
            15 => Some("GIC ITS"),
            16 => Some("Multiprocessor Wakup"),
            17 => Some("Core Programmable Interrupt Controller (CORE PIC)"),
            18 => Some("Legacy I/O Programmable Interrupt Controller (LIO PIC)"),
            19 => Some("HyperTransport Programmable Interrupt Controller (HT PIC)"),
            20 => Some("Extend I/O Programmable Interrupt Controller (EIO PIC)"),
            21 => Some("MSI Programmable Interrupt Controller (MSI PIC)"),
            22 => Some("Bridge I/O Programmable Interrupt Controller (BIO PIC)"),
            23 => Some("Low Pin Count Programmable Interrupt Controller (LPC PIC)"),
            0x18..=0x7F => Some("Reserved"),
            0x80..=0xFF => Some("OEM Specific"),
        }
    }
}
