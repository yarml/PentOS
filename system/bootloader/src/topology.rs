use {
    boot_protocol::topology::{
        Hart, InterruptController, InterruptOverrride, PCIConfigSpace, Topology,
    },
    config::{
        dev::pci::MAX_MCFG_ENTRIES,
        topology::hart::{MAX_HART_COUNT, MAX_INTCTL_COUNT},
    },
    log::debug,
    spinlocks::mutex::{SpinMutex, SpinMutexGuard},
    x64::ioapic::{InputPolarity, TriggerMode},
};

static SYSTEM_TOPOLOGY: SpinMutex<Topology> = SpinMutex::new(Topology::new());

pub fn register_hart(hart: Hart) {
    let mut topology = SYSTEM_TOPOLOGY.lock();
    if topology.harts.push(hart).is_err() {
        complain_big_system("harts", MAX_HART_COUNT);
    }
}

pub fn register_interrupt_controller(interrupt_controller: InterruptController) {
    let mut topology = SYSTEM_TOPOLOGY.lock();
    if topology.int_controllers.push(interrupt_controller).is_err() {
        complain_big_system("interrupt controllers", MAX_INTCTL_COUNT);
    }
}

pub fn register_pci_config_space(cfg_space: PCIConfigSpace) {
    let mut topology = SYSTEM_TOPOLOGY.lock();
    if topology.pci_config_spaces.push(cfg_space).is_err() {
        complain_big_system("PCI configuration spaces", MAX_MCFG_ENTRIES);
    }
}

pub fn register_interrupt_source_override(irq: usize, r#override: InterruptOverrride) {
    let mut topology = SYSTEM_TOPOLOGY.lock();
    if topology.irq_overrides[irq].is_some() {
        panic!("IRQ override for {irq} specified more than once");
    }
    if irq == r#override.gsi
        && r#override.polarity == InputPolarity::ActiveHigh
        && r#override.trigger == TriggerMode::Edge
    {
        return;
    }
    topology.irq_overrides[irq] = Some(r#override);
}

pub fn topology() -> SpinMutexGuard<'static, Topology> {
    SYSTEM_TOPOLOGY.lock()
}

pub fn dump() {
    let topology = SYSTEM_TOPOLOGY.lock();
    debug!("System topology");
    debug!(
        "\tHarts: {found}/{max}",
        found = topology.harts.len(),
        max = MAX_HART_COUNT
    );
    for hart in &topology.harts {
        debug!(
            "\t\tHart#{apic}@{acpi}",
            apic = hart.apic_id,
            acpi = hart.acpi_id
        );
    }
    debug!(
        "\tInterrupt Controllers: {found}/{max}",
        found = topology.int_controllers.len(),
        max = MAX_INTCTL_COUNT
    );
    for int_controller in &topology.int_controllers {
        debug!(
            "\t\tController#{}@{}",
            int_controller.id, int_controller.gsi_base
        );
    }

    debug!("\tIRQ Overrides:");
    for irq in 0..16 {
        if let Some(r#override) = topology.irq_overrides[irq] {
            debug!(
                "\t\tIRQ {irq} -> GSI {} ({:?}:{:?})",
                r#override.gsi, r#override.trigger, r#override.polarity
            );
        }
    }
}

fn complain_big_system(feature: &str, max: usize) -> ! {
    panic!(
        "System has more {feature} than supported kernel configuration. (maximum supported: {max})"
    )
}
