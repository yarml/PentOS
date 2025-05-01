use acpi::table::IOApic;
use acpi::table::InterruptSourceOverride;
use acpi::table::LocalApic;
use acpi::table::Madt;
use acpi::table::MadtEntryHeader;
use acpi::table::Rsdp;
use boot_protocol::acpi::IOApicInfo;
use boot_protocol::acpi::InterruptSourceOverrideInfo;
use boot_protocol::acpi::LocalApicInfo;
use boot_protocol::acpi::MAX_CPU_COUNT;
use boot_protocol::acpi::MAX_IOAPIC_COUNT;
use boot_protocol::acpi::MAX_IS_OVERRIDES;
use common::collections::smallvec::SmallVec;
use log::warn;
use spinlocks::once::Once;
use uefi::system;
use uefi::table;
use x64::mem::addr::PhysAddr;

pub struct AcpiInitInfo {
    pub lapics: SmallVec<LocalApicInfo, MAX_CPU_COUNT>,
    pub ioapics: SmallVec<IOApicInfo, MAX_IOAPIC_COUNT>,
    pub is_overrides: SmallVec<InterruptSourceOverrideInfo, MAX_IS_OVERRIDES>,
}

pub fn init() -> AcpiInitInfo {
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
    if !xsdt.verify() {
        panic!("XSDT table checksum failed");
    }

    let mut lapics = SmallVec::new();
    let mut ioapics = SmallVec::new();
    let mut is_overrides = SmallVec::new();

    for entry in xsdt.entries() {
        if !entry.verify_checksum() {
            panic!("Corrupt ACPI table");
        }

        if &entry.sig == b"APIC" {
            let madt = unsafe { &*(entry as *const _ as *const Madt) };
            // First pass for Local APICs, and I/O Apics
            for entry in madt.iter().filter(|entry| is_lapic_or_ioapic(entry)) {
                match entry.ty {
                    Madt::LOCAL_APIC_TY => {
                        let lapic = unsafe { &*(entry as *const _ as *const LocalApic) };
                        // I couldn't really find in the ACPI spec that APIC ID 255 is invalid, but in FreeBSD they have
                        // MAX_ACPI_ID set to 254
                        // https://lists.freebsd.org/pipermail/freebsd-current/2017-January/064312.html?utm_source=chatgpt.com
                        // And Linux calls 0xFF an invalid ID
                        // https://github.com/torvalds/linux/blob/4f79eaa2ceac86a0e0f304b0bab556cca5bf4f30/arch/x86/kernel/acpi/boot.c#L265C4-L265C5

                        if lapic.apic_id == 255 || lapic.flags | 1 != 1 {
                            continue;
                        }
                        if lapics
                            .push(LocalApicInfo {
                                id: lapic.apic_id as usize,
                                proc_uid: lapic.proc_uid as usize,
                            })
                            .is_err()
                        {
                            complain_big_system("processors", MAX_CPU_COUNT);
                        }
                    }
                    Madt::IO_APIC_TY => {
                        let ioapic = unsafe { &*(entry as *const _ as *const IOApic) };
                        if ioapics
                            .push(IOApicInfo {
                                id: ioapic.ioapic_id as usize,
                                address: PhysAddr::new_truncate(ioapic.address as usize),
                                gsi_base: ioapic.gsi_base as usize,
                            })
                            .is_err()
                        {
                            complain_big_system("I/O APICs", MAX_IOAPIC_COUNT);
                        }
                    }
                    _ => unreachable!(),
                }
            }

            // Second pass for whatever other than LAPIC or IO APIC
            for entry in madt.iter().filter(|entry| !is_lapic_or_ioapic(entry)) {
                match entry.ty {
                    Madt::IS_OVERRIDE_TY => {
                        let is_override =
                            unsafe { &*(entry as *const _ as *const InterruptSourceOverride) };
                        if is_overrides
                            .push(InterruptSourceOverrideInfo {
                                source: is_override.source as usize,
                                gsi: is_override.gsi as usize,
                            })
                            .is_err()
                        {
                            complain_big_system("interrupt source override", MAX_IS_OVERRIDES);
                        }
                    }
                    unsupported => {
                        warn!("Unsupported MADT entry: {unsupported}")
                    }
                }
            }
        }
    }

    AcpiInitInfo {
        lapics,
        ioapics,
        is_overrides,
    }
}

fn is_lapic_or_ioapic<'a>(entry: &'a MadtEntryHeader) -> bool {
    entry.ty == Madt::LOCAL_APIC_TY || entry.ty == Madt::IO_APIC_TY
}

fn complain_big_system(feature: &'static str, max: usize) -> ! {
    panic!(
        "System has more {feature} than supported kernel configuration. (maximum supported: {max})"
    )
}
