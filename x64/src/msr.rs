pub mod efer;

use core::arch::asm;
use core::ops::Deref;
use core::ops::DerefMut;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RawMsr {
    value: u64,
}

impl RawMsr {
    pub fn new(value: u64) -> Self {
        Self { value }
    }
}

impl RawMsr {
    pub fn read(msr: u32) -> Self {
        let low: u32;
        let high: u32;

        unsafe {
            asm!(
                "rdmsr",
                in("ecx") msr,
                out("eax") low,
                out("edx") high,
            );
        }
        let value = ((high as u64) << 32) | (low as u64);
        Self { value }
    }

    pub fn write(&self, msr: u32) {
        let low = (self.value & 0xFFFF_FFFF) as u32;
        let high = (self.value >> 32) as u32;

        unsafe {
            asm!(
                "wrmsr",
                in("ecx") msr,
                in("eax") low,
                in("edx") high,
            );
        }
    }
}

impl Deref for RawMsr {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for RawMsr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
