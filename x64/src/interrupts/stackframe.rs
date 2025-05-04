use crate::mem::addr::VirtAddr;
use crate::mem::segmentation::selector::SegmentSelector;

#[repr(C, packed)]
pub struct InterruptStackFrame {
    pub ip: VirtAddr,
    pub code_selector: SegmentSelector,
    res0: [u8; 6],
    pub rflags: u64,
    pub sp: VirtAddr,
    pub stack_selector: SegmentSelector,
    res1: [u8; 6],
}
