use crate::{
    mem::addr::{Address, VirtAddr},
    msr::RawMsr,
};

const MSR: u32 = 0xC000_0102;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KernelGS {
    raw: RawMsr,
}

impl KernelGS {
    pub fn new(base: VirtAddr) -> Self {
        Self {
            raw: RawMsr {
                value: base.as_u64(),
            },
        }
    }

    pub fn read() -> Self {
        Self {
            raw: RawMsr::read(MSR),
        }
    }
    pub fn write(&self) {
        self.raw.write(MSR)
    }
}

impl KernelGS {
    pub fn get_base(&self) -> VirtAddr {
        VirtAddr::from(*self.raw)
    }
}
