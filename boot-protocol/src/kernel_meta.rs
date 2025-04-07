use x64::mem::addr::VirtAddr;

#[repr(C)]
pub struct KernelMeta {
    pub bsp_entry: VirtAddr,
    pub ap_entry: VirtAddr,
}
