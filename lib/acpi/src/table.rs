mod fadt;
mod gas;
mod header;
mod madt;
mod mcfg;
mod rsdp;
mod xsdt;

use core::mem;
pub use {
    fadt::{Fadt, PmTimerInfo},
    gas::GenericAddress,
    header::AcpiHeader,
    madt::{
        IOApic, InterruptSourceOverride, LocalApic, LocalX2Apic, Madt, MadtEntryHeader,
        MadtIterator,
    },
    mcfg::{ConfigSpacePhysicalMapEntry, Mcfg, McfgIter},
    rsdp::Rsdp,
    xsdt::Xsdt,
};

pub type Signature = &'static [u8; 4];

pub const XSDT_SIG: Signature = b"XSDT";
pub const FADT_SIG: Signature = b"FACP";
pub const MADT_SIG: Signature = b"APIC";
pub const MCFG_SIG: Signature = b"MCFG";

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
