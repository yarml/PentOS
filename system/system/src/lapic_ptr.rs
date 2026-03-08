use x64::lapic::LocalApicPointer;

use crate::vmem::LOCAL_APIC_REGION;

pub fn standard() -> LocalApicPointer {
    unsafe {
        // SAFETY: Technically safety is not guanreteed here, we just trust the caller, without even showing unsafe
        // Reason being this is supposed to be a convenience API
        LocalApicPointer::from_virt_addr(LOCAL_APIC_REGION.start())
    }
}
