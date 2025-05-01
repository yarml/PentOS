use x64::mem::addr::PhysAddr;

pub const MAX_CPU_COUNT: usize = 16;
pub const MAX_IOAPIC_COUNT: usize = 16;
pub const MAX_IS_OVERRIDES: usize = 16; // I believe this is the architectural limit, since this is for ISA IRQ compatibility anyways

#[repr(C)]
pub struct LocalApicInfo {
    pub id: usize,
    pub proc_uid: usize,
}

#[repr(C)]
pub struct IOApicInfo {
    pub id: usize,
    pub address: PhysAddr,
    pub gsi_base: usize,
}

#[repr(C)]
pub struct InterruptSourceOverrideInfo {
    pub source: usize,
    pub gsi: usize,
}
