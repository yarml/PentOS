use {core::arch::asm, x64::mem::segmentation::selector::SegmentSelector};

/// Data structure that contains the information related to each hart (hardware thread).
/// There exists one instance of this strcture for each hart, and each hart's KERNEL_GS
/// points to this structure.
///
/// Assembly code could be accessing this structure using direct offsets, so changing the order of
/// existing fields here should be done with extreme care.
///
/// Using `usize` on all fields, even though some don't need it to keep the structure as simple as possible
#[repr(C)]
pub struct HartInfo {
    pub is_bsp: usize,
    pub osid: usize,
    pub hard_id: usize,
    pub tls_base: usize,
    pub stack: usize,
    pub df_stack: usize,
    pub nmi_stack: usize,
    pub kernel_code_selector: usize,
    pub kernel_data_selector: usize,
    pub user_code_selector: usize,
    pub user_data_selector: usize,
    pub tss_selector: usize,
    pub tss_segment: usize,
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

impl HartInfo {
    pub const fn is_bsp(&self) -> bool {
        self.is_bsp != 0
    }
    pub const fn kernel_code_selector(&self) -> SegmentSelector {
        SegmentSelector::new_raw(self.kernel_code_selector as u16)
    }
    pub const fn kernel_data_selector(&self) -> SegmentSelector {
        SegmentSelector::new_raw(self.kernel_data_selector as u16)
    }
    pub const fn user_code_selector(&self) -> SegmentSelector {
        SegmentSelector::new_raw(self.user_code_selector as u16)
    }
    pub const fn user_data_selector(&self) -> SegmentSelector {
        SegmentSelector::new_raw(self.user_data_selector as u16)
    }
}
