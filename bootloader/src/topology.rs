use boot_protocol::topology::Hart;
use boot_protocol::topology::InterruptController;
use boot_protocol::topology::MAX_HART_COUNT;
use boot_protocol::topology::MAX_INTCTL_COUNT;
use boot_protocol::topology::Topology;
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

fn complain_big_system(feature: &str, max: usize) -> ! {
    panic!(
        "System has more {feature} than supported kernel configuration. (maximum supported: {max})"
    )
}
