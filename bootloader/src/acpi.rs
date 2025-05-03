mod madt;
mod xsdt;

use acpi::table::Madt;
use acpi::table::MadtEntryHeader;
use acpi::table::Rsdp;
use spinlocks::once::Once;
use uefi::system;
use uefi::table;

pub fn init() {
    let rsdp: Once<Option<&Rsdp>> = Once::new();
    system::with_config_table(|table| {
        rsdp.init(|| {
            table
                .iter()
                .find(|entry| entry.guid == table::cfg::ACPI2_GUID)
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

fn is_lapic_or_ioapic<'a>(entry: &'a MadtEntryHeader) -> bool {
    entry.ty == Madt::LOCAL_APIC_TY || entry.ty == Madt::IO_APIC_TY
}

fn complain_corrupt_acpi(info: &str) -> ! {
    panic!("Corrupt ACPI table: {info}")
}
