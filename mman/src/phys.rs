use x64::mem::MemorySize;
use x64::mem::PhysicalMemoryRegion;
use x64::mem::addr::PhysAddr;

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
            .expect("Run out of physical memory")
    }
}
