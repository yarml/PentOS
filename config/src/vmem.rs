//! Non configurable
//! # Memory layout of PentOS
//! Virtual memory is divided into 3 parts
//! - [Userspace](USERSPACE_REGION): Contains currently running process and is different per hart at any given point in time
//! - [Shared Kernel space](KERNEL_SHARED_REGION): Common to all harts, contains the global kernel heap, physical mapping, the kernel, ...
//! - [Local Kernel space](KERNEL_LOCAL_REGION): Contains hart specific data, such as the stack, local APIC mapping, local heap, ...

// TODO: proc macro to do this in a tree structure easily
// I'm dreaming, but maybe I should make a crate which shows this as graphs
// Gonna look good for the capstone

use x64::mem::{
    MemorySize, VirtualMemoryRegion,
    addr::{Address, VirtAddr},
};

pub const MAX_PHYS_SPACE: MemorySize = g(512);

/// Userspace is currently still undefined
pub const USERSPACE_REGION: VirtualMemoryRegion =
    VirtualMemoryRegion::new(VirtAddr::null(), t(128));

/// Kernel space is divided into 2 further parts,
/// [global](KERNEL_SHARED_REGION) and [local](KERNEL_LOCAL_REGION) to each hart.
pub const KERNELSPACE: VirtualMemoryRegion =
    VirtualMemoryRegion::new(VirtAddr::new_panic(0xFFFF800000000000), t(128));

/// Shared kernel space is divided into other parts:
/// - [Physical Mapping](PHYSICAL_MAPPING_REGION)
/// - [Kernel code, data, and rodata](KBIN_REGION)
/// - [Kernel stacks](KSTACK_REGION)
/// - [Global MMIO](GLOBAL_MMIO_REGION)
/// - [Local APIC](LOCAL_APIC_REGION)
pub const KERNEL_SHARED_REGION: VirtualMemoryRegion = firstof(KERNELSPACE, t(64), b(0));

/// Region where data passed from the bootloader to the kernel is stored
pub const BOOTINFO_REGION: VirtualMemoryRegion = firstof(KERNEL_SHARED_REGION, m(1), b(0));

/// Contains 1 to 1 mapping with physical memory.
/// Only maps as much as actually exists, the rest is left unmapped.
/// This memory is always mapped with WriteBack type.
pub const PHYSICAL_MAPPING_REGION: VirtualMemoryRegion =
    firstof(KERNEL_SHARED_REGION, MAX_PHYS_SPACE, b(0));

/// Contains the kernel code, data, and rodata. Only the pages used are actually mapped.
/// This memory is always mapped with WriteBack type.
/// This needs to always be synchronized with the kernel's link.ld script
/// otherwise the bootloader will refuse to load the kernel
pub const KBIN_REGION: VirtualMemoryRegion = after(PHYSICAL_MAPPING_REGION, g(16), b(0), b(0));

/// Main kernel stack region. Limited by [KSTACK_SIZE](crate::topology::hart::KSTACK_SIZE).
/// Always WriteBack memoty type.
pub const KSTACK_REGION: VirtualMemoryRegion = after(KBIN_REGION, g(8), b(0), b(0));

/// Reserved for future expansions to kernel execution requirements (e.g. hart local memory).
pub const KRESERVED_REGION: VirtualMemoryRegion = after(KSTACK_REGION, g(488), b(0), b(0));

/// Contains global tables that can be accessed by all harts, this includes the GDT,
/// a copy of the common paging entries here as a source of truth so that harts will copy
/// them into their paging structure whenever they synchronize.
pub const SYSTAB_REGION: VirtualMemoryRegion = after(KRESERVED_REGION, g(512), b(0), b(0));

/// Used by drivers which provide global MMIO devices. The framebuffer is mapped here.
/// The memory type of any page is determined by the driver in question, and the allocator responsible
/// for this region allows specifying any memory type.
pub const GLOBAL_MMIO_REGION: VirtualMemoryRegion = after(SYSTAB_REGION, t(1), b(0), b(0));

/// Local APIC
pub const LOCAL_APIC_REGION: VirtualMemoryRegion = after(GLOBAL_MMIO_REGION, k(4), b(0), b(0));

/// Like global kernel space, this is also divded into parts.
pub const KERNEL_LOCAL_REGION: VirtualMemoryRegion = after(KERNEL_SHARED_REGION, t(64), b(0), b(0));

/// Hart local heap. Only pages actually used are mapped. Always uses the WriteBack memory type.
/// Subject to swapping policies.
pub const LOCAL_HEAP_REGION: VirtualMemoryRegion = firstof(KERNEL_LOCAL_REGION, g(512), b(0));

/// Used by drivers which provide MMIO that should only be accessed through a single hart.
pub const LOCAL_MMIO_REGION: VirtualMemoryRegion = firstof(KERNEL_LOCAL_REGION, t(1), b(0));

const fn after(
    prev: VirtualMemoryRegion,
    size: MemorySize,
    distance: MemorySize,
    subtract: MemorySize,
) -> VirtualMemoryRegion {
    VirtualMemoryRegion::new(
        prev.start()
            .add_panic(prev.size().as_usize())
            .add_panic(distance.as_usize()),
        MemorySize::new(size.as_usize() - subtract.as_usize()),
    )
}

const fn firstof(
    parent: VirtualMemoryRegion,
    size: MemorySize,
    offset: MemorySize,
) -> VirtualMemoryRegion {
    VirtualMemoryRegion::new(parent.start().add_panic(offset.as_usize()), size)
}

const fn b(n: usize) -> MemorySize {
    MemorySize::new(n)
}
const fn k(n: usize) -> MemorySize {
    const K1: usize = 0x1000;
    MemorySize::new(n * K1)
}
const fn m(n: usize) -> MemorySize {
    const M1: usize = 0x100000;
    MemorySize::new(n * M1)
}
const fn g(n: usize) -> MemorySize {
    const G1: usize = 0x40000000;
    MemorySize::new(n * G1)
}
const fn t(n: usize) -> MemorySize {
    const T1: usize = 0x10000000000;
    MemorySize::new(n * T1)
}
