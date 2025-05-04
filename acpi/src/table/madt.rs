use super::AcpiHeader;
use super::AcpiTable;
use super::MADT_SIG;
use super::Signature;
use core::marker::PhantomData;
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

#[repr(C, packed)]
pub struct IOApic {
    pub header: MadtEntryHeader,
    pub ioapic_id: u8,
    pub res0: u8,
    pub address: u32,
    pub gsi_base: u32,
}

#[repr(C, packed)]
pub struct InterruptSourceOverride {
    pub header: MadtEntryHeader,
    pub bus: u8,
    pub source: u8,
    pub gsi: u32,
    pub flags: u16,
}

pub struct MadtIterator<'a> {
    madt: &'a Madt,
    cursor: usize,
}

pub struct MadtFilteredIterator<'a, T: MadtEntry> {
    iterator: MadtIterator<'a>,
    _phantom: PhantomData<T>,
}

impl Madt {
    pub const LOCAL_APIC_TY: u8 = 0;
    pub const IO_APIC_TY: u8 = 1;
    pub const IS_OVERRIDE_TY: u8 = 2;
    pub const LOCAL_APIC_NMI_TY: u8 = 4;
}

impl Madt {
    pub fn iter(&self) -> MadtIterator {
        MadtIterator {
            madt: self,
            cursor: mem::size_of::<Madt>(),
        }
    }

    pub fn entries<T: MadtEntry>(&self) -> MadtFilteredIterator<'_, T> {
        MadtFilteredIterator {
            iterator: MadtIterator {
                madt: self,
                cursor: mem::size_of::<Madt>(),
            },
            _phantom: PhantomData,
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

impl<'a, T: 'a + MadtEntry> Iterator for MadtFilteredIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        for entry in self.iterator.by_ref() {
            if entry.ty == T::TYPE {
                // Unwrapping then wrapping again to avoid
                // a corrupt ACPI being seen as end of iterator
                return Some(entry.getas().expect("Corrupt ACPI"));
            }
        }
        None
    }
}

impl MadtEntryHeader {
    pub fn getas<T: MadtEntry>(&self) -> Option<&T> {
        if self.ty == T::TYPE {
            Some(unsafe {
                // # Safety
                // If type checks out, and this is still unsafe,
                // it is the vendor's problem, not mine
                &*(self as *const _ as *const T)
            })
        } else {
            None
        }
    }
}

impl AcpiTable for Madt {
    const SIG: Signature = MADT_SIG;
}

pub trait MadtEntry {
    const TYPE: u8;
}

impl MadtEntry for LocalApic {
    const TYPE: u8 = Madt::LOCAL_APIC_TY;
}

impl MadtEntry for IOApic {
    const TYPE: u8 = Madt::IO_APIC_TY;
}

impl MadtEntry for InterruptSourceOverride {
    const TYPE: u8 = Madt::IS_OVERRIDE_TY;
}
