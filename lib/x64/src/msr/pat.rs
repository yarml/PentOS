use {super::RawMsr, crate::mem::paging::pat::PatIndex};

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
    pub fn new() -> Self {
        Self {
            raw: RawMsr::new(0),
        }
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

impl Default for Pat {
    fn default() -> Self {
        Self::new()
    }
}
