use crate::mem::paging::pat::PatIndex;

use super::RawMsr;

const MSR: u32 = 0x277;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Pat {
    raw: RawMsr,
}

#[derive(Clone, Copy)]
pub enum MemoryType {
    Uncacheable,
    WriteCombining,
    WriteThrough,
    WriteProtected,
    WriteBack,
    Uncached,
}

impl Pat {
    /// Same as power-up or reset state (Intel 64 Architecture, Volume 3A, Section 13.12.4, December 2024)
    pub fn new() -> Self {
        let mut val = Self {
            raw: RawMsr::new(0),
        };
        val.set(PatIndex::new(0), MemoryType::WriteBack)
            .set(PatIndex::new(1), MemoryType::WriteThrough)
            .set(PatIndex::new(2), MemoryType::Uncached)
            .set(PatIndex::new(3), MemoryType::Uncacheable)
            .set(PatIndex::new(4), MemoryType::WriteBack)
            .set(PatIndex::new(5), MemoryType::WriteThrough)
            .set(PatIndex::new(6), MemoryType::Uncached)
            .set(PatIndex::new(7), MemoryType::Uncacheable);
        val
    }
    pub fn write(&self) {
        self.raw.write(MSR);
    }
}

impl Pat {
    pub fn set(&mut self, index: PatIndex, mtype: MemoryType) -> &mut Self {
        let raw = mtype.raw();
        *self.raw &= !(0b111 << (*index * 8));
        *self.raw |= (raw as u64) << (*index * 8);
        self
    }
}

impl MemoryType {
    pub fn raw(&self) -> u8 {
        match self {
            MemoryType::Uncacheable => 0x00,
            MemoryType::WriteCombining => 0x01,
            MemoryType::WriteThrough => 0x04,
            MemoryType::WriteProtected => 0x05,
            MemoryType::WriteBack => 0x06,
            MemoryType::Uncached => 0x07,
        }
    }
}

/// PatIndex assuming the correct standard table is setup from standard_pat() -> Pat
pub fn pat_index(mtype: MemoryType) -> PatIndex {
    match mtype {
        MemoryType::Uncacheable => PatIndex::new(2),
        MemoryType::WriteCombining => PatIndex::new(3),
        MemoryType::WriteThrough => PatIndex::new(1),
        MemoryType::WriteProtected => PatIndex::new(4),
        MemoryType::WriteBack => PatIndex::new(0),
        MemoryType::Uncached => PatIndex::new(5),
    }
}

pub fn standard_pat() -> Pat {
    let mut val = Pat::new();
    val.set(PatIndex::new(0), MemoryType::WriteBack)
        .set(PatIndex::new(1), MemoryType::WriteThrough)
        .set(PatIndex::new(2), MemoryType::Uncacheable)
        .set(PatIndex::new(3), MemoryType::WriteCombining)
        .set(PatIndex::new(4), MemoryType::WriteProtected)
        .set(PatIndex::new(5), MemoryType::Uncached)
        .set(PatIndex::new(6), MemoryType::Uncacheable)
        .set(PatIndex::new(7), MemoryType::WriteBack);
    val
}

impl Default for Pat {
    fn default() -> Self {
        Self::new()
    }
}
