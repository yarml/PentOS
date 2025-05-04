pub mod addr;
pub mod frame;
pub mod page;
pub mod paging;
pub mod segmentation;

mod region;
mod size;

pub use region::PhysicalMemoryRegion;
pub use region::VirtualMemoryRegion;
pub use size::MemorySize;
