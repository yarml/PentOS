#![no_std]

pub mod features;

use x64::mem::MemoryRegion;

const MMAP_PG_COUNT: usize = 1;
pub const MAX_MMAP_SIZE: usize = MMAP_PG_COUNT * (4096 / core::mem::size_of::<MemoryRegion>());

#[repr(C)]
pub struct BootInfo {
    pub mmap: [MemoryRegion; MAX_MMAP_SIZE],
    pub mmap_len: usize,
}
