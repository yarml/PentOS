use crate::mem::addr::Address;
use crate::mem::addr::PhysAddr;
use crate::mem::frame::Frame;
use crate::mem::frame::size::Frame2MiB;
use crate::mem::frame::size::Frame4KiB;
use crate::mem::page::size::Page1GiB;
use crate::mem::page::size::Page2MiB;
use crate::mem::page::size::Page4KiB;
use crate::mem::page::size::PageSize;
use crate::mem::paging::PagingMapEntry;
use crate::mem::paging::PagingReferenceEntry;

#[test]
fn test_map_entry_pte() {
    let frame = Frame::<Frame4KiB>::containing(PhysAddr::new_panic(0x1000));
    let entry = PagingMapEntry::<Page4KiB>::new(frame);
    assert_eq!(*entry & 1, 1); // Present bit set
    assert_eq!(*entry & Page4KiB::USE_MAP_FLAG, 0); // No PS bit
}

#[test]
fn test_map_entry_pde() {
    let frame = Frame::<Frame2MiB>::containing(PhysAddr::new_panic(0x200000));
    let entry = PagingMapEntry::<Page2MiB>::new(frame);
    assert_eq!(*entry & 1, 1); // Present bit set
    assert_eq!(*entry & Page2MiB::USE_MAP_FLAG, 1 << 7); // PS bit set
}

#[test]
#[should_panic]
fn test_invalid_map_entry_pde() {
    // Missing PS bit (bit 7)
    PagingMapEntry::<Page2MiB>::from_inner(0x1);
}

#[test]
fn test_reference_entry() {
    let frame = Frame::<Frame4KiB>::containing(PhysAddr::new_panic(0x3000));
    let entry = PagingReferenceEntry::<Page1GiB>::new(frame);
    assert_eq!(*entry & 1, 1); // Present bit set
    assert_eq!(*entry & Page1GiB::USE_MAP_FLAG, 0); // PS bit cleared
}

#[test]
#[should_panic]
fn test_invalid_reference_entry() {
    // PS bit (bit 7) set
    PagingReferenceEntry::<Page1GiB>::from_inner(0x81);
}
