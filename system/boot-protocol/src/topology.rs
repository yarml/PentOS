use {
    config::{
        dev::pci::MAX_MCFG_ENTRIES,
        topology::hart::{MAX_HART_COUNT, MAX_INTCTL_COUNT},
    },
    utils::collections::smallvec::SmallVec,
    x64::{
        ioapic::{InputPolarity, TriggerMode},
        mem::addr::PhysAddr,
    },
};

#[derive(Clone)]
#[repr(C)]
pub struct Topology {
    pub harts: SmallVec<Hart, MAX_HART_COUNT>,
    pub int_controllers: SmallVec<InterruptController, MAX_INTCTL_COUNT>,
    pub irq_overrides: [Option<InterruptOverrride>; 16],
    pub pci_config_spaces: SmallVec<PCIConfigSpace, MAX_MCFG_ENTRIES>,
}

// Too proud of myself to call this CPU
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Hart {
    pub apic_id: usize,
    pub acpi_id: usize,
}

// Too proud of myself to call this IO APIC
#[derive(Clone, Copy)]
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
            irq_overrides: [None; 16],
            pci_config_spaces: SmallVec::new(),
        }
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct InterruptOverrride {
    pub gsi: usize,
    pub polarity: InputPolarity,
    pub trigger: TriggerMode,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PCIConfigSpace {
    pub phys_base: PhysAddr,
    pub segment_group: usize,
    pub bus_start: usize,
    pub bus_end: usize,
}
