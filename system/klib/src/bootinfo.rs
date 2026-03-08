use {boot_protocol::BootInfo, system::vmem::BOOTINFO_REGION, x64::mem::addr::Address};

pub fn bootinfo() -> &'static BootInfo {
    unsafe {
        // SAFETY: Guarenteed by caller
        &*BOOTINFO_REGION.start().as_ptr::<BootInfo>()
    }
}
