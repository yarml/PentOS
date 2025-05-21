use x64::mem::MemorySize;
use x64::mem::PhysicalMemoryRegion;
use x64::mem::addr::PhysAddr;

#[derive(Default)]
pub struct PhysicalAllocationRequest {
    pub size: MemorySize,
    pub alignment: Option<MemorySize>,
    pub below: Option<PhysAddr>,
    pub continuous: bool,
}

pub trait PhysicalMemoryAllocator {
    fn maybe_allocate(&self, req: PhysicalAllocationRequest) -> Option<PhysicalMemoryRegion>;

    fn free(&self, region: PhysicalMemoryRegion);

    fn allocate(&self, req: PhysicalAllocationRequest) -> PhysicalMemoryRegion {
        self.maybe_allocate(req)
            .expect("Ran out of physical memory")
    }
}

impl PhysicalAllocationRequest {
    pub const fn size_align(size: MemorySize, align: MemorySize) -> Self {
        Self {
            size,
            alignment: Some(align),
            below: None,
            continuous: false,
        }
    }
}
