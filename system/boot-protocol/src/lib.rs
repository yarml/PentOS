#![no_std]

pub mod features;
pub mod kernel_init;
pub mod topology;

use {features::FeatureSet, system::framebuffer::FramebufferInfo, x64::mem::PhysicalMemoryRegion};

const MMAP_PG_COUNT: usize = 1;
pub const MAX_MMAP_SIZE: usize =
    MMAP_PG_COUNT * (4096 / core::mem::size_of::<PhysicalMemoryRegion>());

pub const STACK_SIZE: usize = 512 * 0x1000;

#[repr(C, align(4096))]
pub struct BootInfo {
    pub mmap: [PhysicalMemoryRegion; MAX_MMAP_SIZE],
    pub mmap_len: usize,
    pub features: FeatureSet,
    pub framebuffer: FramebufferInfo,
}
