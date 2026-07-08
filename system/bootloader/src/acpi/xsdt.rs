use {
    super::{complain_corrupt_acpi, madt},
    crate::{acpi::mcfg, timers},
    acpi::table::{AcpiTable, Fadt, Madt, Mcfg, Xsdt},
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
    let mcfg = xsdt.find_unique::<Mcfg>();

    let pm_timer_info = fadt.pm_timer_info();

    madt::parse(madt);
    mcfg::parse(mcfg);
    timers::init_pm(pm_timer_info);
}
