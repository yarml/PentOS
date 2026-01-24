mod madt;
mod xsdt;

use {
    acpi::table::{Madt, MadtEntryHeader, Rsdp},
    spinlocks::once::Once,
    uefi::{system, table::cfg::ConfigTableEntry},
};

pub fn init() {
    let rsdp: Once<Option<&Rsdp>> = Once::new();
    system::with_config_table(|table| {
        rsdp.init(|| {
            table
                .iter()
                .find(|entry| entry.guid == ConfigTableEntry::ACPI2_GUID)
                .map(|entry| unsafe { &*(entry.address as *const Rsdp) })
        });
    });

    // ifta7 ya sim sim
    let Some(Some(rsdp)) = rsdp.get().cloned() else {
        panic!("ACPI2 table not found");
    };

    if !rsdp.verify() {
        panic!("RSDP table checksum failed");
    }
    if rsdp.revivion != 2 {
        panic!(
            "Unsupported RSDP revision {revision}",
            revision = rsdp.revivion
        );
    }
    let xsdt = rsdp.xsdt();
    xsdt::parse(xsdt);
}

fn is_lapic_or_ioapic(entry: &MadtEntryHeader) -> bool {
    entry.ty == Madt::LOCAL_APIC_TY || entry.ty == Madt::IO_APIC_TY
}

fn complain_corrupt_acpi(info: &str) -> ! {
    panic!("Corrupt ACPI table: {info}")
}
