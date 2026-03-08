use crate::mem::addr::VirtAddr;

#[derive(Clone, Copy)]
#[repr(C, packed(4))]
pub struct TaskStateSegment {
    res0: u32,
    pub rsp: [VirtAddr; 3],
    res1: u32,
    res2: u32,
    pub ist: [VirtAddr; 7],
    res3: u32,
    res4: u32,
    res5: u16,
    iopbm: u16,
}
