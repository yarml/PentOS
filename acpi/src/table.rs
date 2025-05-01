mod fadt;
mod gas;
mod header;
mod madt;
mod rsdp;
mod xsdt;

pub use gas::GenericAddress;
pub use header::AcpiHeader;
pub use madt::IOApic;
pub use madt::InterruptSourceOverride;
pub use madt::LocalApic;
pub use madt::Madt;
pub use madt::MadtEntryHeader;
pub use madt::MadtIterator;
pub use rsdp::Rsdp;
pub use xsdt::Xsdt;

use core::mem;

pub type Signature = &'static [u8; 4];

pub const XSDT_SIG: Signature = b"XSDT";
pub const FADT_SIG: Signature = b"FACP";
pub const MADT_SIG: Signature = b"APIC";

pub trait AcpiTable: Sized {
    const SIG: &'static [u8; 4];

    fn get_header(&self) -> &AcpiHeader {
        unsafe { &*(self as *const _ as *const AcpiHeader) }
    }

    fn verify(&self) -> bool {
        let header = self.get_header();
        &header.sig == Self::SIG
            && header.verify_checksum()
            && header.len as usize >= mem::size_of::<Self>()
    }
}
