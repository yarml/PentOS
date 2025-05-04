use acpi::table::IOApic;
use acpi::table::LocalApic;
use acpi::table::Madt;
use boot_protocol::topology::Hart;
use boot_protocol::topology::InterruptController;
use x64::mem::addr::Address;
use x64::mem::addr::PhysAddr;

use crate::topology::register_hart;
use crate::topology::register_interrupt_controller;

pub fn parse(madt: &Madt) {
    madt.entries::<LocalApic>().for_each(parse_lapic);
    madt.entries::<IOApic>().for_each(parse_ioapic);
}

fn parse_lapic(lapic: &LocalApic) {
    // I couldn't really find in the ACPI spec that APIC ID 255 is invalid,
    // but in FreeBSD they have MAX_ACPI_ID set to 254
    // https://lists.freebsd.org/pipermail/freebsd-current/2017-January/064312.html?utm_source=chatgpt.com
    // And Linux calls 0xFF an invalid ID
    // https://github.com/torvalds/linux/blob/4f79eaa2ceac86a0e0f304b0bab556cca5bf4f30/arch/x86/kernel/acpi/boot.c#L265C4-L265C5
    if lapic.apic_id == 255 || (lapic.flags & 1 != 1 && lapic.flags & 2 != 1) {
        return;
    }
    register_hart(Hart {
        apic_id: lapic.apic_id as usize,
        acpi_id: lapic.proc_uid as usize,
    });
}

fn parse_ioapic(ioapic: &IOApic) {
    register_interrupt_controller(InterruptController {
        id: ioapic.ioapic_id as usize,
        register_base: PhysAddr::new_panic(ioapic.address as usize),
        gsi_base: ioapic.gsi_base as usize,
    });
}
