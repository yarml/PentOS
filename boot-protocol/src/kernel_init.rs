use x64::mem::addr::VirtAddr;

pub type KernelInitFn = extern "C" fn() -> KernelEntryInfo;

#[repr(C)]
pub struct KernelEntryInfo {
    pub bsp_entry: VirtAddr,
    pub ap_entry: VirtAddr,
}
