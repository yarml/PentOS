use acpi::table::LocalApic;
use acpi::table::Madt;
use acpi::table::Rsdp;
use boot_protocol::MAX_CPU_COUNT;
use common::collections::smallvec::SmallVec;
use log::debug;
use spinlocks::once::Once;
use uefi::system;
use uefi::table;

pub struct AcpiInitInfo {
    pub lapics: SmallVec<LocalApicInfo, MAX_CPU_COUNT>,
}

#[derive(Clone, Copy, Debug)]
pub struct LocalApicInfo {
    pub id: usize,
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
    let str_signature = xsdt.header.signature();
    if let Some(sig) = str_signature {
        debug!("XSDT signature: {sig}");
    } else {
        debug!("XSDT signature: {:?}", xsdt.header.sig);
    }

    let mut lapics = SmallVec::new();

    for entry in xsdt.entries() {
        if !entry.verify_checksum() {
            panic!("Corrupt ACPI table");
        }
        let str_signature = entry.signature();
        if let Some(sig) = str_signature {
            debug!("Found ACPI table: {sig}");
        } else {
            debug!("Found ACPI table: {:?}", entry.sig);
        }

        if &entry.sig == b"APIC" {
            let madt = unsafe { &*(entry as *const _ as *const Madt) };
            for entry in madt {
                debug!("\t{ty}", ty = entry.ty);
                if entry.ty == 0 {
                    let lapic = unsafe { &*(entry as *const _ as *const LocalApic) };
                    debug!("\t\tProcessor: {proc}", proc = lapic.proc_uid);
                    debug!("\t\tID: {id}", id = lapic.apic_id);
                    if lapics
                        .push(LocalApicInfo {
                            id: lapic.apic_id as usize,
                        })
                        .is_err()
                    {
                        panic!(
                            "System has more processors than supported by current kernel configuration. (maximum supported: {MAX_CPU_COUNT})"
                        );
                    }
                }
            }
        }
    }

    AcpiInitInfo { lapics }
}
