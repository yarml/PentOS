#![no_std]

pub mod features;
pub mod framebuffer;
pub mod kernel_meta;
pub mod topology;

use features::FeatureSet;
use framebuffer::FramebufferInfo;
use x64::mem::PhysicalMemoryRegion;

const MMAP_PG_COUNT: usize = 1;
pub const MAX_MMAP_SIZE: usize = MMAP_PG_COUNT * (4096 / core::mem::size_of::<PhysicalMemoryRegion>());

pub const OFFSET_MAPPING: usize = 0xFFFF800000000000;

pub const STACK_SIZE: usize = 512 * 1024;

#[repr(C, align(4096))]
pub struct BootInfo {
    pub mmap: [PhysicalMemoryRegion; MAX_MMAP_SIZE],
    pub mmap_len: usize,
    pub features: FeatureSet,
    pub framebuffer: FramebufferInfo,
}
