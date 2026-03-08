use {
    super::{complain_corrupt_acpi, madt}, crate::timers, acpi::table::{AcpiTable, Fadt, Madt, Xsdt}
};

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
    let fadt = xsdt.find_unique::<Fadt>();
    madt::parse(madt);

    let pm_timer_info = fadt.pm_timer_info();
    timers::init_pm(pm_timer_info);
}
