use common::collections::smallvec::SmallVec;
use config::topology::hart::MAX_HART_COUNT;
use config::topology::hart::MAX_INTCTL_COUNT;
use x64::mem::addr::PhysAddr;

#[repr(C)]
pub struct Topology {
    pub harts: SmallVec<Hart, MAX_HART_COUNT>,
    pub int_controllers: SmallVec<InterruptController, MAX_INTCTL_COUNT>,
}

// Too proud of myself to call this CPU
#[repr(C)]
pub struct Hart {
    pub apic_id: usize,
    pub acpi_id: usize,
}

// Too proud of myself to call this IO APIC
#[repr(C)]
pub struct InterruptController {
    pub id: usize,
    pub register_base: PhysAddr,
    pub gsi_base: usize,
}

impl Topology {
    pub const fn new() -> Self {
        Self {
            harts: SmallVec::new(),
            int_controllers: SmallVec::new(),
        }
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}
