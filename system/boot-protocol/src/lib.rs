#![no_std]

pub mod features;
pub mod kernel_init;
pub mod topology;

use {
    crate::topology::Topology, features::FeatureSet, system::framebuffer::FramebufferInfo,
    x64::mem::PhysicalMemoryRegion,
};

const MMAP_PG_COUNT: usize = 1;
pub const MAX_MMAP_SIZE: usize =
    MMAP_PG_COUNT * (4096 / core::mem::size_of::<PhysicalMemoryRegion>());

#[repr(C, align(4096))]
pub struct BootInfo {
    pub mmap: [PhysicalMemoryRegion; MAX_MMAP_SIZE],
    pub mmap_len: usize,
    pub features: FeatureSet,
    pub topology: Topology,
    pub framebuffer: FramebufferInfo,
}
