use {
    crate::mem::addr::{Address, VirtAddr},
    core::ptr,
};

#[derive(Clone, Copy)]
pub struct IoApic {
    base: VirtAddr,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum IoApicRegister {
    Id = 0x00,
    Version = 0x01,
    ArbitrationId = 0x02,
}

#[derive(Clone, Copy)]
pub struct IoApicVersion {
    pub version: u8,
    pub redirection_count: u8,
}

#[derive(Clone, Copy)]
pub enum IoRedirection {
    FixedPhysical {
        vector: u8,
        apic_id: u8,
        trigger: TriggerMode,
        polarity: InputPolarity,
        mask: bool,
    },
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TriggerMode {
    Edge = 0,
    Level = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InputPolarity {
    ActiveHigh = 0,
    ActiveLow = 1,
}

const IOREGSEL_OFFSET: usize = 0x00;
const IOWIN_OFFSET: usize = 0x10;
const IOREDTBL_BASE: u8 = 0x10;

impl IoApic {
    /// Construct an `IoApic` from a virtual address pointing to IOREGSEL.
    ///
    /// # Safety
    /// - `base` must be mapped as Uncacheable MMIO for at least 0x14 bytes.
    /// - This must be the only `IoApic` instance using this base address,
    ///   for the lifetime of the instance.
    pub const unsafe fn new(base: VirtAddr) -> Self {
        Self { base }
    }

    #[inline(always)]
    fn ioregsel(&self) -> *mut u32 {
        (self.base + IOREGSEL_OFFSET).as_mut_ptr()
    }

    #[inline(always)]
    fn iowin(&self) -> *mut u32 {
        (self.base + IOWIN_OFFSET).as_mut_ptr()
    }

    pub fn read(&mut self, reg: u8) -> u32 {
        unsafe {
            ptr::write_volatile(self.ioregsel(), reg as u32);
            ptr::read_volatile(self.iowin())
        }
    }

    pub fn write(&mut self, reg: u8, value: u32) {
        unsafe {
            ptr::write_volatile(self.ioregsel(), reg as u32);
            ptr::write_volatile(self.iowin(), value);
        }
    }

    pub fn id(&mut self) -> u8 {
        ((self.read(IoApicRegister::Id as u8) >> 24) & 0xF) as u8
    }

    pub fn version(&mut self) -> IoApicVersion {
        let reg = self.read(IoApicRegister::Version as u8);
        IoApicVersion {
            version: (reg & 0xFF) as u8,
            redirection_count: ((reg >> 16) & 0xFF) as u8 + 1,
        }
    }

    pub fn read_redirection(&mut self, index: u8) -> usize {
        let reg = IOREDTBL_BASE + index * 2;
        let low = self.read(reg) as usize;
        let high = self.read(reg + 1) as usize;
        low | high << 32
    }

    pub fn write_redirection(&mut self, index: u8, redirection: IoRedirection) {
        let vector: usize = match redirection {
            IoRedirection::FixedPhysical { vector, .. } => vector,
            IoRedirection::Disabled => 0,
        } as usize;
        let deliv_mode = match redirection {
            IoRedirection::FixedPhysical { .. } => 0b000,
            IoRedirection::Disabled => 0b000,
        };
        let dest_mode = match redirection {
            IoRedirection::FixedPhysical { .. } => 0,
            IoRedirection::Disabled => 0,
        };
        let polarity = match redirection {
            IoRedirection::FixedPhysical { polarity, .. } => polarity,
            IoRedirection::Disabled => InputPolarity::ActiveHigh,
        } as usize;
        let trigger = match redirection {
            IoRedirection::FixedPhysical { trigger, .. } => trigger,
            IoRedirection::Disabled => TriggerMode::Edge,
        } as usize;
        let masked = match redirection {
            IoRedirection::FixedPhysical { mask, .. } => mask,
            IoRedirection::Disabled => true,
        } as usize;
        let dest = match redirection {
            IoRedirection::FixedPhysical { apic_id, .. } => {
                if apic_id & 0xF != apic_id {
                    panic!("apic id too large")
                } else {
                    apic_id
                }
            }
            IoRedirection::Disabled => 0,
        } as usize;

        let value = (dest << 56)
            | (masked << 16)
            | (trigger << 15)
            | (polarity << 13)
            | (dest_mode << 11)
            | (deliv_mode << 8)
            | vector;

        let reg = IOREDTBL_BASE + index * 2;
        let low = (value & 0xFFFF_FFFF) as u32;
        let high = ((value >> 32) & 0xFFFF_FFFF) as u32;
        self.write(reg, low);
        self.write(reg + 1, high);
    }
}
