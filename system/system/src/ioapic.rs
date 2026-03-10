use {
    crate::vmem::IOAPIC_REGION,
    x64::mem::{
        addr::VirtAddr,
        frame::size::{Frame4KiB, FrameSize},
    },
};

pub fn standard_addressof(id: usize) -> VirtAddr {
    IOAPIC_REGION.start() + id * Frame4KiB::SIZE
}
