use crate::mem::addr::VirtAddr;
use core::arch::x86_64::__cpuid;
use core::hint;
use core::ptr;

pub fn id_cpuid() -> usize {
    (unsafe {
        // SAFETY: nothing to worry about
        __cpuid(1)
    }
    .ebx >> 24) as usize
}

pub struct LocalApicPointer {
    pointer: VirtAddr,
}

pub struct LocalApicVersion {
    pub version: usize,
    pub lvt_count: usize,
    pub supress_eoi_ability: bool,
}

pub struct InterProcessorInterrupt {
    pub delivery_mode: IPIDeliveryMode,
    pub destination_mode: IPIDestinationMode,
    pub destination: IPIDestination,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum IPIDeliveryMode {
    Fixed { vector: u8 } = 0b000,
    SMI = 0b010,
    NMI = 0b100,
    Init { level: IPILevel } = 0b101,
    StartUp { vector: u8 } = 0b110,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum IPIDestinationMode {
    Physical = 0,
    Logical = 1,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum IPILevel {
    Deassert { trigger: IPITriggerMode } = 0,
    Assert = 1,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum IPITriggerMode {
    Edge = 0,
    Level = 1,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum IPIDestination {
    Explicit { tartget_apicid: u8 } = 0b00,
    OnlySelf = 0b01,
    EveryoneAndSelf = 0b10,
    EveryoneExceptSelf = 0b11,
}

#[repr(usize)]
pub enum LocalApicRegister {
    ID = 0x20,
    Version = 0x30,
    ICRLow = 0x300,
    ICRHigh = 0x310,
}

impl LocalApicPointer {
    pub fn read_reg32(&self, reg: LocalApicRegister) -> u32 {
        unsafe {
            // # Safety
            // This should be safe since each hart can only access their own Local APIC.
            ptr::read_volatile((self.pointer + reg as usize).as_ptr())
        }
    }
    pub fn write_reg32(&self, reg: LocalApicRegister, value: u32) {
        unsafe {
            // # Safety
            // This should be safe since each hart can only access thei own Local APIC.
            ptr::write_volatile((self.pointer + reg as usize).as_mut_ptr(), value);
        };
    }

    pub fn id(&self) -> usize {
        self.read_reg32(LocalApicRegister::ID) as usize >> 24
    }
    pub fn version(&self) -> LocalApicVersion {
        let reg = self.read_reg32(LocalApicRegister::Version) as usize;
        let version = reg & 0xF;
        let lvt_count = (reg >> 16) & 0xFF + 1;
        let supress_eoi_ability = (reg >> 24) & 1 == 1;
        LocalApicVersion {
            version,
            lvt_count,
            supress_eoi_ability,
        }
    }

    pub fn send_ipi(&self, ipi: InterProcessorInterrupt) {
        let destination_field = match ipi.destination {
            IPIDestination::Explicit { tartget_apicid } => tartget_apicid,
            _ => 0,
        };
        let (vector, level, trigger_mode) = match ipi.delivery_mode {
            IPIDeliveryMode::Fixed { vector } | IPIDeliveryMode::StartUp { vector } => {
                (vector, 1, 0)
            }
            IPIDeliveryMode::Init {
                level: IPILevel::Deassert { trigger },
            } => (0, 0, trigger as u8),
            IPIDeliveryMode::Init {
                level: IPILevel::Assert,
            } => (0, 1, 0),
            _ => (0, 1, 0),
        };
        let delivery_mode = ipi.delivery_mode.discriminant();
        let destination_mode = ipi.destination_mode as u8;
        let destination_shorthand = ipi.destination.discriminant();

        let upper_dword = (destination_field as u32) >> 24;
        let lower_dword = (vector as u32) << 0
            | (delivery_mode as u32) << 8
            | (destination_mode as u32) << 11
            | (level as u32) << 14
            | (trigger_mode as u32) << 15
            | (destination_shorthand as u32) << 18;

        self.write_reg32(LocalApicRegister::ICRHigh, upper_dword);
        self.write_reg32(LocalApicRegister::ICRHigh, lower_dword);

        while self.read_reg32(LocalApicRegister::ICRLow) & (1 << 12) == 1 {
            hint::spin_loop();
        }
    }
}

impl IPIDeliveryMode {
    pub fn discriminant(&self) -> u8 {
        unsafe {
            // # Safety
            // Safe as per: https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.discriminant.access-memory
            *(self as *const _ as *const u8)
        }
    }
}

impl IPIDestination {
    pub fn discriminant(&self) -> u8 {
        unsafe {
            // # Safety
            // Safe as per: https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.discriminant.access-memory
            *(self as *const _ as *const u8)
        }
    }
}
