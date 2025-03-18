use crate::mem::addr::PhysAddr;

#[test]
fn test_addr_creation() {
    let valid_addr = PhysAddr::new(0x00FF_FFFF_FFFF_FFFF);
    assert!(valid_addr.is_some());

    let truncated = PhysAddr::new_truncate(0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(truncated.as_usize(), 0x00FF_FFFF_FFFF_FFFF);
}

#[test]
fn test_arithmetic() {
    let mut addr = PhysAddr::new(0x1000).unwrap();
    addr += 0x500;
    assert_eq!(addr.as_usize(), 0x1500);

    addr -= 0x300;
    assert_eq!(addr.as_usize(), 0x1200);
}

#[test]
fn test_conversions() {
    let addr: PhysAddr = (0x1234 as usize).into();
    assert_eq!(addr.as_usize(), 0x1234);

    let u64_val: u64 = addr.into();
    assert_eq!(u64_val, 0x1234);
}

#[test]
fn test_subtraction() {
    let a = PhysAddr::new(0x2000).unwrap();
    let b = PhysAddr::new(0x1000).unwrap();
    let diff = a - b;
    assert_eq!(*diff, 0x1000);
}
