use {
    super::RawMsr,
    crate::mem::{
        addr::{Address, PhysAddr},
        frame::{Frame, size::Frame4KiB},
    },
    core::ops::Deref,
};

const MSR: u32 = 0x1B;

const BSP_BIT: usize = 8;
const X2APIC_BIT: usize = 10;
const ENABLED_BIT: usize = 11;

const BASE_MASK: usize = !0xFFF;

pub const STANDARD_PHYS_BASE: PhysAddr = PhysAddr::new_panic(0xFEE00000);

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ApicBase {
    raw: RawMsr,
}

impl ApicBase {
    pub fn read() -> Self {
        Self {
            raw: RawMsr::read(MSR),
        }
    }

    /// # Safety
    /// Must guarentee that the new configuration will not cause an undefined behaviour
    /// in rust
    pub unsafe fn write(self) {
        self.raw.write(MSR);
    }
}

impl ApicBase {
    pub fn is_enabled(&self) -> bool {
        (**self >> ENABLED_BIT) & 1 == 1
    }
    pub fn is_x2apic(&self) -> bool {
        (**self >> X2APIC_BIT) & 1 == 1
    }
    pub fn is_bsp(&self) -> bool {
        (**self >> BSP_BIT) & 1 == 1
    }
    pub fn phys_base(&self) -> Frame<Frame4KiB> {
        Frame::containing(PhysAddr::new_panic(**self as usize & BASE_MASK))
    }
}

impl ApicBase {
    pub fn with_enabled(self, enabled: bool) -> ApicBase {
        let mask = 1 << ENABLED_BIT;
        ApicBase {
            raw: RawMsr::new(if enabled { *self | mask } else { *self & !mask }),
        }
    }

    pub fn with_phys_base(self, phys_base: Frame<Frame4KiB>) -> ApicBase {
        ApicBase {
            raw: RawMsr::new((*self & !BASE_MASK as u64) | phys_base.boundary().as_u64()),
        }
    }
}

impl Deref for ApicBase {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
