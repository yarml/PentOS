use {crate::BootInfo, x64::mem::addr::VirtAddr};

pub type KernelInitFn = extern "sysv64" fn(&BootInfo) -> KernelEntryInfo;

#[repr(C)]
pub struct KernelEntryInfo {
    pub bsp_entry: VirtAddr,
    pub ap_entry: VirtAddr,
}
