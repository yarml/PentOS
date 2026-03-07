use core::arch::asm;

/// Data structure that contains the information related to each hart (hardware thread).
/// There exists one instance of this strcture for each hart, and each hart's KERNEL_GS
/// points to this structure.
///
/// Assembly code could be accessing this structure using direct offsets, so changing the order of
/// existing fields here should be done with extreme care.
#[repr(C)]
pub struct HartInfo {
    pub is_bsp: bool,
    pub osid: usize,
    pub hard_id: usize,
    pub tls_base: usize,
    pub stack: usize,
}

impl HartInfo {
    pub fn get() -> &'static Self {
        let location: *const Self;
        unsafe {
            asm! {
                "rdgsbase {loc}",
                loc = out(reg) location
            };
        }
        unsafe { &*location }
    }
}
