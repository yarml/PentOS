use {
    crate::topology::register_pci_config_space,
    acpi::table::Mcfg,
    boot_protocol::topology::PCIConfigSpace,
    x64::mem::addr::{Address, PhysAddr},
};

pub fn parse(mcfg: &Mcfg) {
    for cs in mcfg {
        register_pci_config_space(PCIConfigSpace {
            phys_base: PhysAddr::new_panic(cs.base as usize),
            segment_group: cs.seg_group as usize,
            bus_start: cs.bus_start as usize,
            bus_end: cs.bus_end as usize,
        });
    }
}
