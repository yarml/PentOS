//! Non configurable
//! # Physical memory layout
//! Physical memory is divided into 4 parts
//! - [Super low memory](SUPERLOWMEM)
//! - [Low memory](LOWMEM)
//! - [Middle memory](MIDMEM)
//! - [High memory](HIGHMEM)

use x64::mem::MemorySize;
use x64::mem::PhysicalMemoryRegion;
use x64::mem::addr::Address;
use x64::mem::addr::PhysAddr;

const M1: MemorySize = MemorySize::new(1024 * 1024);
const M16: MemorySize = MemorySize::new(16 * M1.as_usize());
const G4: MemorySize = MemorySize::new(256 * M16.as_usize());
const G512: MemorySize = MemorySize::new(128 * G4.as_usize());

/// Memory below 1 MiB. Used by bootloader to bootstrap secondary processors. The kernel ignores this memory.
pub const SUPERLOWMEM: PhysicalMemoryRegion = PhysicalMemoryRegion::new(PhysAddr::null(), M1);

/// Memory below 16 MiB. Intended for devices which need DMA but do not support 32 bit addressing.
pub const LOWMEM: PhysicalMemoryRegion = PhysicalMemoryRegion::new(
    PhysAddr::new_panic(M1.as_usize()),
    MemorySize::new(M16.as_usize() - M1.as_usize()),
);

/// Memory between 16 Mib and 4 GiB. Prioritized for DMA devices which support 32 bit addressing, but can otherwise also be used for general pupose use.
pub const MIDMEM: PhysicalMemoryRegion = PhysicalMemoryRegion::new(
    PhysAddr::new_panic(M16.as_usize()),
    MemorySize::new(G4.as_usize() - M16.as_usize()),
);

/// Memory above 4 GiB, until 512 GiB. Any memory above that will be ignored by the kernel(limitation). For general purpose use, and in case a device supports above 32 bit addressing.
pub const HIGHMEM: PhysicalMemoryRegion = PhysicalMemoryRegion::new(
    PhysAddr::new_panic(G4.as_usize()),
    MemorySize::new(G512.as_usize() - G4.as_usize()),
);
