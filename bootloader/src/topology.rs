use boot_protocol::topology::Hart;
use boot_protocol::topology::InterruptController;
use boot_protocol::topology::MAX_HART_COUNT;
use boot_protocol::topology::MAX_INTCTL_COUNT;
use boot_protocol::topology::Topology;
use log::debug;
use spinlocks::mutex::Mutex;

static SYSTEM_TOPOLOGY: Mutex<Topology> = Mutex::new(Topology::new());

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
}

fn complain_big_system(feature: &str, max: usize) -> ! {
    panic!(
        "System has more {feature} than supported kernel configuration. (maximum supported: {max})"
    )
}
