use super::complain_corrupt_acpi;
use super::madt;
use acpi::table::AcpiTable;
use acpi::table::Madt;
use acpi::table::Xsdt;

pub fn parse(xsdt: &Xsdt) {
    if !xsdt.verify() {
        complain_corrupt_acpi("Invalid XSDT");
    }
    for entry in xsdt {
        if !entry.verify_checksum() {
            complain_corrupt_acpi("Invalid XSDT entry");
        }
    }

    let madt = xsdt.find_unique::<Madt>();
    madt::parse(madt);
}
