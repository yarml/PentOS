pub mod handlers;

use {
    crate::{bootinfo, interrupts::VECTOR_PS2_KEYBOARD},
    boot_protocol::topology::InterruptOverrride,
    config::topology::hart::MAX_INTCTL_COUNT,
    log::debug,
    spinlocks::{mutex::Mutex, once::Once},
    system::ioapic,
    utils::collections::smallvec::{SmallVec, SmallVecBuf},
    x64::ioapic::{InputPolarity, IoApic, IoRedirection, TriggerMode},
};

const DEFAULT_PS2_KEYBOARD_IRQ: usize = 1;

static INT_CONTROLLERS: Once<SmallVec<IoApicContainer, MAX_INTCTL_COUNT>> = Once::new();

struct IoApicContainer {
    ioapic: Mutex<IoApic>,
    // id: usize,
    gsi_base: usize,
    count: usize,
}

pub(crate) fn init() {
    let bootinfo = bootinfo::bootinfo();

    let interrupt_controllers = &bootinfo.topology.int_controllers;

    let io_hart = bootinfo
        .topology
        .harts
        .iter()
        .find(|hart| hart.apic_id < 16)
        .expect("could not find a hart with an ID suitable for I/O APIC")
        .apic_id;

    debug!("Using hart #{io_hart} for I/O APIC interrupts");

    let mut int_controllers = SmallVec::new();

    // Initialize all I/O APIC entries to masked:
    for controller in interrupt_controllers {
        let mut ioapic = unsafe {
            // Guarenteed by caller
            IoApic::new(ioapic::standard_addressof(controller.id))
        };
        let count = ioapic.version().redirection_count as usize;
        let gsi_base = controller.gsi_base;

        for i in 0..count {
            ioapic.write_redirection(i as u8, IoRedirection::Disabled);
        }

        let container = IoApicContainer {
            ioapic: Mutex::new(ioapic),
            count,
            gsi_base,
            // id: controller.id,
        };
        if int_controllers.push(container).is_err() {
            panic!("not enough interrupt controller slots");
        }
    }

    int_controllers.sort_by_key(|c1| c1.gsi_base);

    apply_irq_redirection(
        io_hart,
        DEFAULT_PS2_KEYBOARD_IRQ,
        VECTOR_PS2_KEYBOARD,
        &mut int_controllers,
    );

    INT_CONTROLLERS.init(|| int_controllers);
}

fn apply_irq_redirection(
    io_hart_apic_id: usize,
    irq: usize,
    vector: u8,
    int_controllers: &mut SmallVecBuf<IoApicContainer>,
) {
    let bootinfo = bootinfo::bootinfo();
    let irq_overrides = &bootinfo.topology.irq_overrides;
    let redirection = irq_overrides[irq].unwrap_or(InterruptOverrride {
        gsi: irq,
        polarity: InputPolarity::ActiveHigh,
        trigger: TriggerMode::Edge,
    });

    if let Some(gsi_controller) = int_controllers.iter().find(|controller| {
        controller.gsi_base <= redirection.gsi
            && redirection.gsi < controller.gsi_base + controller.count
    }) {
        let mut ioapic = gsi_controller.ioapic.lock();
        ioapic.write_redirection(
            (redirection.gsi - gsi_controller.gsi_base) as u8,
            IoRedirection::FixedPhysical {
                vector,
                apic_id: io_hart_apic_id as u8,
                trigger: redirection.trigger,
                polarity: redirection.polarity,
                mask: false,
            },
        );
    }
}
