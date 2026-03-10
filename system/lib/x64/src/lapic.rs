use crate::msr::RawMsr;

pub struct LocalApic;

#[derive(Clone, Copy)]
pub struct LocalApicVersion {
    pub version: usize,
    pub lvt_count: usize,
    pub supress_eoi_ability: bool,
}

#[derive(Clone, Copy)]
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
    Explicit { tartget_apicid: usize } = 0b00,
    OnlySelf = 0b01,
    EveryoneAndSelf = 0b10,
    EveryoneExceptSelf = 0b11,
}

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum LocalApicRegister {
    ID = 0x20,
    Version = 0x30,
    EndOfInterrupt = 0xB0,
    SpuriousVector = 0xF0,
    ICR = 0x300,
    LVTTimer = 0x320,
    LVTError = 0x0370,
    InitCount = 0x380,
    CurrentCount = 0x390,
    DivConf = 0x3E0,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum TimerMode {
    OneShot = 0b00,
    Periodic = 0b01,
    TSCDeadline = 0b10,
}

impl LocalApic {
    #[inline(always)]
    pub fn read_reg(reg: LocalApicRegister) -> usize {
        let msr = 0x800 + (reg as u32 >> 4);
        *RawMsr::read(msr) as usize
    }
    #[inline(always)]
    pub fn write_reg(reg: LocalApicRegister, value: usize) {
        let msr = 0x800 + (reg as u32 >> 4);
        RawMsr::new(value as u64).write(msr)
    }

    #[inline(always)]
    pub fn id() -> usize {
        Self::read_reg(LocalApicRegister::ID)
    }
    #[inline(always)]
    pub fn version() -> LocalApicVersion {
        let reg = Self::read_reg(LocalApicRegister::Version);
        let version = reg & 0xF;
        let lvt_count = ((reg >> 16) & 0xFF) + 1;
        let supress_eoi_ability = (reg >> 24) & 1 == 1;
        LocalApicVersion {
            version,
            lvt_count,
            supress_eoi_ability,
        }
    }

    pub fn send_ipi(ipi: InterProcessorInterrupt) {
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

        let icr = (vector as usize)
            | (delivery_mode as usize) << 8
            | (destination_mode as usize) << 11
            | (level as usize) << 14
            | (trigger_mode as usize) << 15
            | (destination_shorthand as usize) << 18
            | destination_field << 32;

        Self::write_reg(LocalApicRegister::ICR, icr);
    }

    #[inline(always)]
    pub fn set_timer_divisor(divisor: u8) {
        let divconf: usize = match divisor {
            1 => 0b1011,
            2 => 0b0000,
            4 => 0b0001,
            8 => 0b0010,
            16 => 0b0011,
            32 => 0b1000,
            64 => 0b1001,
            128 => 0b1010,
            _ => panic!("Invalid LAPIC timer divisor {divisor}"),
        };
        Self::write_reg(LocalApicRegister::DivConf, divconf)
    }

    #[inline(always)]
    pub fn set_timer_initial(value: u32) {
        Self::write_reg(LocalApicRegister::InitCount, value as usize)
    }
    #[inline(always)]
    pub fn get_timer() -> u32 {
        Self::read_reg(LocalApicRegister::CurrentCount) as u32
    }

    #[inline(always)]
    pub fn program_spurious_vector(vector: u8) {
        Self::write_reg(LocalApicRegister::SpuriousVector, vector as usize | 0x100)
    }
    #[inline(always)]
    pub fn program_lvt_timer(vector: u8, mode: TimerMode) {
        Self::write_reg(
            LocalApicRegister::LVTTimer,
            vector as usize | (mode as usize) << 17,
        )
    }
    #[inline(always)]
    pub fn program_lvt_error(vector: u8) {
        Self::write_reg(LocalApicRegister::LVTError, vector as usize)
    }

    #[inline(always)]
    pub fn end_of_interrupt() {
        Self::write_reg(LocalApicRegister::EndOfInterrupt, 0);
    }
}

impl IPIDeliveryMode {
    pub fn discriminant(&self) -> u8 {
        unsafe {
            // SAFETY: Safe as per: https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.discriminant.access-memory
            *(self as *const _ as *const u8)
        }
    }
}

impl IPIDestination {
    pub fn discriminant(&self) -> u8 {
        unsafe {
            // SAFETY: Safe as per: https://doc.rust-lang.org/reference/items/enumerations.html#r-items.enum.discriminant.access-memory
            *(self as *const _ as *const u8)
        }
    }
}
