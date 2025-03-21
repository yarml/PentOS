use crate::mem::MemorySize;
use alloc::format;

#[test]
fn test_size_creation() {
    let size = MemorySize::new(1024);
    assert_eq!(*size, 1024);
    assert_eq!(*MemorySize::zero(), 0);
}

#[test]
fn test_arithmetic() {
    let mut a = MemorySize::new(10);
    a += MemorySize::new(20);
    assert_eq!(*a, 30);

    a -= 5;
    assert_eq!(*a, 25);

    a *= 2;
    assert_eq!(*a, 50);
}

#[test]
fn test_display() {
    let size = MemorySize::new(1025);
    assert_eq!(format!("{}", size), "1K1B");

    let zero = MemorySize::zero();
    assert_eq!(format!("{}", zero), "0B");
}

#[test]
fn test_debug() {
    let size = MemorySize::new(0x400000);
    assert_eq!(format!("{:?}", size), "0000E0000P0000T0000G0004M0000K0000B");
}
