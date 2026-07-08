use {
    crate::topology::{
        register_hart, register_interrupt_controller, register_interrupt_source_override,
    },
    acpi::table::{IOApic, InterruptSourceOverride, LocalApic, LocalX2Apic, Madt},
    boot_protocol::topology::{Hart, InterruptController, InterruptOverrride},
    x64::{
        ioapic::{InputPolarity, TriggerMode},
        mem::addr::{Address, PhysAddr},
    },
};

pub fn parse(madt: &Madt) {
    madt.entries::<LocalApic>().for_each(parse_lapic);
    madt.entries::<LocalX2Apic>().for_each(parse_x2apic);
    madt.entries::<IOApic>().for_each(parse_ioapic);
    madt.entries::<InterruptSourceOverride>()
        .for_each(parse_is_override);
}

fn parse_lapic(lapic: &LocalApic) {
    // I couldn't really find in the ACPI spec that APIC ID 255 is invalid,
    // but in FreeBSD they have MAX_ACPI_ID set to 254
    // https://lists.freebsd.org/pipermail/freebsd-current/2017-January/064312.html?utm_source=chatgpt.com
    // And Linux calls 0xFF an invalid ID
    // https://github.com/torvalds/linux/blob/4f79eaa2ceac86a0e0f304b0bab556cca5bf4f30/arch/x86/kernel/acpi/boot.c#L265C4-L265C5

    // Okay months later, found it:
    // >  Logical processors
    //    with APIC ID values 255 and greater must use the Processor Local x2APIC structure and be declared using the
    //    Device() keyword.
    // - ACPI specs, page 142, under Processor Local x2APIC Structure section
    if lapic.apic_id == 255 || (lapic.flags & 1 == 0 && lapic.flags & 2 == 0) {
        return;
    }
    register_hart(Hart {
        apic_id: lapic.apic_id as usize,
        acpi_id: lapic.proc_uid as usize,
    });
}

fn parse_x2apic(x2apic: &LocalX2Apic) {
    if x2apic.flags & 1 == 0 && x2apic.flags & 2 == 0 {
        return;
    }
    register_hart(Hart {
        apic_id: x2apic.x2apic_id as usize,
        acpi_id: x2apic.proc_uid as usize,
    });
}

fn parse_ioapic(ioapic: &IOApic) {
    register_interrupt_controller(InterruptController {
        id: ioapic.ioapic_id as usize,
        register_base: PhysAddr::new_panic(ioapic.address as usize),
        gsi_base: ioapic.gsi_base as usize,
    });
}

fn parse_is_override(is_override: &InterruptSourceOverride) {
    if is_override.bus != 0 {
        panic!("Unknown interrupt source override bus: {}", is_override.bus);
    }

    let raw_polarity = is_override.flags & 0b11;
    let raw_trigger = (is_override.flags >> 2) & 0b11;

    let polarity = match raw_polarity {
        0b00 | 0b01 => InputPolarity::ActiveHigh,
        0b11 => InputPolarity::ActiveLow,
        _ => panic!("invalid interrupt override polarity"),
    };

    let trigger = match raw_trigger {
        0b00 | 0b01 => TriggerMode::Edge,
        0b11 => TriggerMode::Level,
        _ => panic!("invalid interrupt override trigger"),
    };

    register_interrupt_source_override(
        is_override.source as usize,
        InterruptOverrride {
            gsi: is_override.gsi as usize,
            polarity,
            trigger,
        },
    );
}
