// All written by deepseek

use crate::mem::MemoryRegion;
use crate::mem::MemorySize;
use crate::mem::addr::PhysAddr;
use alloc::format;
use alloc::vec;

// Helper functions to create test objects
fn addr(n: usize) -> PhysAddr {
    PhysAddr::new_truncate(n)
}

fn memsize(n: usize) -> MemorySize {
    MemorySize::new(n)
}

fn region(start: usize, size: usize) -> MemoryRegion {
    MemoryRegion::new(addr(start), memsize(size))
}

#[test]
fn test_operators() {
    let mut r1 = region(1000, 500);
    let r2 = region(1500, 500);

    // Test Add operator
    let combined = r1 + r2;
    assert_eq!(combined, Some(region(1000, 1000)));

    // Test BitAnd operator
    let overlap = region(1200, 1000) & region(1500, 1000);
    assert_eq!(overlap, region(1500, 700));

    // Test AddAssign
    r1 += r2;
    assert_eq!(r1, region(1000, 1000));

    // Test BitAndAssign
    let mut r3 = region(0, 2000);
    r3 &= region(1000, 1000);
    assert_eq!(r3, region(1000, 1000));
}

#[test]
fn test_ordering() {
    let regions = vec![region(3000, 500), region(1000, 1000), region(2000, 2000)];

    let mut sorted = regions.clone();
    sorted.sort();

    assert_eq!(
        sorted,
        vec![region(1000, 1000), region(2000, 2000), region(3000, 500),]
    );
}

#[test]
fn test_edge_cases() {
    // Helper to create masked address
    fn masked(n: usize) -> usize {
        n & 0x00FFFFFFFFFFFFFF
    }

    // Zero-sized region
    let zero = region(1000, 0);
    assert!(zero.is_null());
    assert!(!zero.contains(addr(1000)));

    // Test PhysAddr truncation
    let high_addr = addr(0xFF123456789ABCDE);
    assert_eq!(*high_addr, masked(0xFF123456789ABCDE));

    // Region at upper boundary without wrap-around
    let max_phys = masked(0x00FFFFFFFFFFFFFF);
    let valid_region = region(max_phys - 500, 500);
    assert_eq!(valid_region.start().as_usize(), max_phys - 500);
    assert_eq!(valid_region.end().as_usize(), max_phys);
    assert!(valid_region.contains(addr(max_phys - 500)));
    assert!(valid_region.contains(addr(max_phys - 1)));
    assert!(!valid_region.contains(addr(max_phys))); // Exclusive end

    // Region wrapping around due to overflow
    let wrap_start = max_phys - 100;
    let wrapping_region = region(wrap_start, 200);
    // Calculate expected end after truncation
    let expected_end = masked(wrap_start + 200);
    assert_eq!(wrapping_region.end().as_usize(), expected_end);

    // Contains checks (region wraps, so start > end)
    assert!(!wrapping_region.contains(addr(wrap_start))); // start <= addr < end → false
    assert!(!wrapping_region.contains(addr(max_phys)));
    assert!(!wrapping_region.contains(addr(expected_end - 1)));

    // Single-byte region at max address
    let edge_region = region(max_phys, 1);
    assert_eq!(edge_region.end().as_usize(), masked(max_phys + 1));
    assert!(!edge_region.contains(addr(max_phys))); // end wraps to 0
}

#[test]
fn test_display_debug() {
    let r = region(0x1000, 0x500);
    assert_eq!(format!("{}", r), "0000000000001000 - 0000000000001500");
    assert_eq!(
        format!("{:?}", r),
        "MemoryRegion(0000000000001000 - 0000000000001500)"
    );
}

#[test]
fn test_compound_operations() {
    // Test chained operations
    let base = region(0, 0x1000);
    let first_half = region(0, 0x800);
    let second_half = region(0x800, 0x800);

    // Union of adjacent halves should make whole
    assert_eq!(first_half + second_half, Some(base));

    // Intersection of overlapping regions
    let overlap = region(0x500, 0x1000) & region(0x800, 0x1000);
    assert_eq!(overlap, region(0x800, 0xD00));

    // Complex operation: ((A ∪ B) ∩ C)
    let a = region(0x1000, 0x500);
    let b = region(0x1500, 0x500);
    let c = region(0x1200, 0x1000);
    let result = (a + b).unwrap() & c;
    assert_eq!(result, region(0x1200, 0x800));
}

#[test]
fn test_constructors() {
    let start = PhysAddr::new(0x1000).unwrap();
    let size = MemorySize::new(0x1000);
    let region = MemoryRegion::new(start, size);
    assert_eq!(region.start(), start);
    assert_eq!(region.size(), size);
    assert_eq!(region.end(), start.add_truncate(0x1000));

    // Test new_boundaries
    let end = start.add_truncate(0x1000);
    let region2 = MemoryRegion::new_boundaries(start, end);
    assert_eq!(region2.size(), size);
}

#[test]
#[should_panic]
fn test_invalid_boundaries() {
    let start = PhysAddr::new(0x2000).unwrap();
    let end = PhysAddr::new(0x1000).unwrap();
    let _region = MemoryRegion::new_boundaries(start, end);
}

#[test]
fn test_contains() {
    let region = MemoryRegion::new(PhysAddr::new(0x1000).unwrap(), MemorySize::new(0x1000));
    assert!(region.contains(PhysAddr::new(0x1000).unwrap()));
    assert!(region.contains(PhysAddr::new(0x1FFF).unwrap()));
    assert!(!region.contains(PhysAddr::new(0x2000).unwrap()));
}

#[test]
fn test_union() {
    let r1 = MemoryRegion::new(PhysAddr::new(0x1000).unwrap(), MemorySize::new(0x1000));
    let r2 = MemoryRegion::new(PhysAddr::new(0x3000).unwrap(), MemorySize::new(0x1000));
    assert_eq!(r1.union(r2), None); // Disjoint

    let r3 = MemoryRegion::new(PhysAddr::new(0x1800).unwrap(), MemorySize::new(0x1000));
    let union = r1.union(r3).unwrap();
    assert_eq!(union.start(), PhysAddr::new(0x1000).unwrap());
    assert_eq!(union.end(), PhysAddr::new(0x2800).unwrap());
}

#[test]
fn test_intersect() {
    let r1 = MemoryRegion::new(PhysAddr::new(0x1000).unwrap(), MemorySize::new(0x1000));
    let r2 = MemoryRegion::new(PhysAddr::new(0x1800).unwrap(), MemorySize::new(0x1000));
    let intersection = r1.intersect(r2);
    assert_eq!(intersection.start(), PhysAddr::new(0x1800).unwrap());
    assert_eq!(intersection.end(), PhysAddr::new(0x2000).unwrap());
}

#[test]
fn take_start_partial() {
    let mut region = MemoryRegion::new(PhysAddr::new(0x1000).unwrap(), MemorySize::new(0x1000));
    let taken = region.take_start(0x500);
    assert_eq!(taken, PhysAddr::new(0x1000).unwrap());
    assert_eq!(region.start(), PhysAddr::new(0x1500).unwrap());
    assert_eq!(*region.size(), 0xB00);
}

#[test]
fn take_start_full() {
    let mut region = MemoryRegion::new(PhysAddr::new(0x2000).unwrap(), MemorySize::new(0x1000));
    let taken = region.take_start(0x1000);
    assert_eq!(taken, PhysAddr::new(0x2000).unwrap());
    assert!(region.is_null());
}

#[test]
#[should_panic] // In debug mode only
fn take_start_over_size() {
    let mut region = MemoryRegion::new(PhysAddr::new(0x3000).unwrap(), MemorySize::new(0x500));
    region.take_start(0x1000); // Panics in debug due to underflow
}

#[test]
fn take_start_zero() {
    let mut region = MemoryRegion::new(PhysAddr::new(0x4000).unwrap(), MemorySize::new(0x1000));
    let taken = region.take_start(0);
    assert_eq!(taken, PhysAddr::new(0x4000).unwrap());
    assert_eq!(*region.size(), 0x1000);
}

#[test]
#[should_panic]
fn take_start_from_null() {
    let mut region = MemoryRegion::null();
    region.take_start(0x1000); // Will panic due to underflow
}
