use {
    boot_protocol::{kernel_init::KernelEntryInfo, BootInfo},
    x64::mem::addr::{Address, VirtAddr},
};

#[unsafe(no_mangle)]
extern "C" fn init(bootinfo: BootInfo) -> KernelEntryInfo {
    klib::init(&bootinfo);

    KernelEntryInfo {
        bsp_entry: VirtAddr::new_panic(crate::entry::bsp_entry as usize),
        ap_entry: VirtAddr::new_panic(crate::entry::ap_entry as usize),
    }
}
