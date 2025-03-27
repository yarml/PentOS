use crate::define_addr;

define_addr!(Addr, make_canonical);

#[inline]
const fn make_canonical(addr: usize) -> usize {
    addr & 0x00FF_FFFF_FFFF_FFFF
}

#[test]
fn test_addr_creation() {
    let valid_addr = Addr::new(0x00FF_FFFF_FFFF_FFFF);
    assert!(valid_addr.is_some());

    let truncated = Addr::new_truncate(0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(truncated.as_usize(), 0x00FF_FFFF_FFFF_FFFF);
}

#[test]
fn test_arithmetic() {
    let mut addr = Addr::new(0x1000).unwrap();
    addr += 0x500;
    assert_eq!(addr.as_usize(), 0x1500);

    addr -= 0x300;
    assert_eq!(addr.as_usize(), 0x1200);
}

#[test]
fn test_conversions() {
    let addr: Addr = (0x1234 as usize).into();
    assert_eq!(addr.as_usize(), 0x1234);

    let u64_val: u64 = addr.into();
    assert_eq!(u64_val, 0x1234);

    let ptr: *const u8 = addr.as_ptr();
    assert_eq!(ptr as usize, 0x1234);

    let mut_ptr: *mut u8 = addr.as_mut_ptr();
    assert_eq!(mut_ptr as usize, 0x1234);
}

#[test]
fn test_subtraction() {
    let a = Addr::new(0x2000).unwrap();
    let b = Addr::new(0x1000).unwrap();
    let diff = a - b;
    assert_eq!(*diff, 0x1000);
}
