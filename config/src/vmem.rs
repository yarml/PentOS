//! Non configurable
//! # Memory layout of PentOS
//! Virtual memory is divided into 3 parts
//! - Userspace: Contains currently running process and is different per hart at any given point in time
//! - Shared Kernel space: Common to all harts, contains the global kernel heap, physical mapping, the kernel, ...
//! - Local Kernel space: Contains hart specific data, such as the stack, local APIC mapping, local heap, ...

// TODO: proc macro to do this in a tree structure easily

use x64::mem::MemorySize;
use x64::mem::VirtualMemoryRegion;
use x64::mem::addr::Address;
use x64::mem::addr::VirtAddr;

const B0: MemorySize = MemorySize::zero();
const G8: MemorySize = MemorySize::new(0x200000000);
const G16: MemorySize = MemorySize::new(2 * G8.as_usize());
const G512: MemorySize = MemorySize::new(32 * G16.as_usize());
const T1: MemorySize = MemorySize::new(2 * G512.as_usize());
const T64: MemorySize = MemorySize::new(64 * T1.as_usize());
const T128: MemorySize = MemorySize::new(128 * T1.as_usize());

/// Userspace is currently still undefined
pub const USERSPACE_REGION: VirtualMemoryRegion = VirtualMemoryRegion::new(VirtAddr::null(), T128);

/// Kernel space is divided into 2 further parts,
/// [global](KERNEL_SHARED_REGION) and [local](KERNEL_LOCAL_REGION) to each hart.
pub const KERNELSPACE: VirtualMemoryRegion =
    VirtualMemoryRegion::new(VirtAddr::new_panic(0xFFFF800000000000), T128);

/// Shared kernel space is divided into other parts:
/// - [Physical Mapping](PHYSICAL_MAPPING_REGION)
/// - [Kernel code, data, and rodata](KBIN_REGION)
/// - [Global kernel heap](GLOBAL_KHEAP_REGION)
pub const KERNEL_SHARED_REGION: VirtualMemoryRegion = firstof(KERNELSPACE, T64, B0);

/// Contains 1 to 1 mapping with physical memory.
/// Only maps as much as actually exists, the rest is left unmapped.
/// This memory is always mapped with WriteBack type.
pub const PHYSICAL_MAPPING_REGION: VirtualMemoryRegion =
    firstof(KERNEL_SHARED_REGION, MAX_PHYS_SPACE, B0);
pub const MAX_PHYS_SPACE: MemorySize = G512;

/// Contains the kernel code, data, and rodata. Only the pages used are actually mapped.
/// This memory is always mapped with WriteBack type.
pub const KBIN_REGION: VirtualMemoryRegion = after(PHYSICAL_MAPPING_REGION, G16, B0, B0);

/// Global kernel heap
/// Only pages actually used are mapped. Pages here always mapped with WriteBack type.
/// Pages here are subject to swapping policies.
pub const GLOBAL_KHEAP_REGION: VirtualMemoryRegion = after(KBIN_REGION, G512, B0, G16);

/// Contains global information that can be accessed by all harts, this includes information about system topology,
/// the GDT. We also keep a copy of the common paging entries here as a source of truth so that harts will copy
/// them into their paging structure whenever they synchronize.
///
/// Bootloader information is also kept here.
pub const SYSINFO_REGION: VirtualMemoryRegion = after(GLOBAL_KHEAP_REGION, G512, B0, B0);

/// This part is used by the memory manager, it contains the bookkeeping sturctures for physical memory
/// allocation, swapping info, ...
pub const MMAN_REGION: VirtualMemoryRegion = after(SYSINFO_REGION, G512, B0, B0);

/// Used by drivers which provide global MMIO devices. The framebuffer is mapped here.
/// The memory type of any page is determined by the driver in question, and the allocator responsible
/// for this region allows specifying any memory type.
pub const GLOBAL_MMIO_REGION: VirtualMemoryRegion = after(MMAN_REGION, T1, B0, B0);

/// Like global kernel space, this is also divded into parts.
pub const KERNEL_LOCAL_REGION: VirtualMemoryRegion = after(KERNEL_SHARED_REGION, T64, B0, B0);

/// Main kernel stack region. Limited by [KSTACK_SIZE](crate::topology::hart::KSTACK_SIZE).
/// Always WriteBack memoty type. Not subject to swapping policies.
pub const KSTACK_REGION: VirtualMemoryRegion = firstof(KERNEL_LOCAL_REGION, G8, B0);

/// Special purpose stacks, such as double fault stack and NMI stack.
/// Limited by [SPECIAL_KSTACK_SIZE](crate::topology::hart::SPECIAL_KSTACK_SIZE).
/// Always WriteBack memory type. Not subject to swapping policies.
pub const SPECIAL_KSTACK_REGION: VirtualMemoryRegion = after(KSTACK_REGION, G8, B0, B0);

/// Hart local heap. Only pages actually used are mapped. Always uses the WriteBack memory type.
/// Subject to swapping policies.
pub const LOCAL_HEAP_REGION: VirtualMemoryRegion = after(SPECIAL_KSTACK_REGION, G512, B0, G16);

/// Used by drivers which provide MMIO that should only be accessed through a single hart.
///
/// Local APIC is mapped here.
pub const LOCAL_MMIO_REGION: VirtualMemoryRegion = after(LOCAL_HEAP_REGION, T1, B0, B0);

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
