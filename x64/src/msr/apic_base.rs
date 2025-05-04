use core::ops::Deref;

use super::RawMsr;

const MSR: u32 = 0x1B;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ApicBase {
    raw: RawMsr,
}

impl ApicBase {
    pub fn get() -> Self {
        Self {
            raw: RawMsr::read(MSR),
        }
    }
}

impl Deref for ApicBase {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
