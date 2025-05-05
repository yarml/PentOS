use super::PagingReferenceEntry;
use super::pat::ReferencePatIndex;
use super::pcid::Pcid;
use crate::mem::addr::Address;
use crate::mem::addr::PhysAddr;
use crate::mem::frame::Frame;
use crate::mem::frame::size::Frame4KiB;
use crate::mem::frame::size::FrameSize;
use crate::mem::page::size::Page4KiB;
use crate::mem::page::size::Page512GiB;
use core::arch::asm;
use core::ops::Deref;

/// This must be considered like a `&mut [PaginReferenceEntry<Page512GiB>]`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PagingRootEntry {
    value: u64,
}

impl PagingRootEntry {
    #[inline]
    pub const fn new(target_frame: Frame<Frame4KiB>) -> Self {
        Self {
            value: target_frame.boundary().as_u64(),
        }
    }
}

/// `PaginRootEntry` is hart dependent and should not be shared
impl !Send for PagingRootEntry {}
/// `PaginRootEntry` is hart dependent and should not be shared
impl !Sync for PagingRootEntry {}

impl PagingRootEntry {
    #[inline]
    pub const fn with_pat_index(self, index: ReferencePatIndex) -> Self {
        Self {
            value: self.value & !(0b11 << 2) | index.pgentry_flags(),
        }
    }
    #[inline]
    pub const fn with_target(self, target_frame: Frame<Frame4KiB>) -> Self {
        Self {
            value: (self.value & !(Frame4KiB::MASK & PhysAddr::MASK) as u64)
                | target_frame.boundary().as_u64(),
        }
    }
    #[inline]
    pub const fn with_pcid(self, pcid: Pcid) -> Self {
        Self {
            value: self.value & !(0xFFF) | (pcid.unwrap() as u64 & 0xFFF),
        }
    }
}

impl PagingRootEntry {
    #[inline]
    pub const fn pat_index(&self) -> ReferencePatIndex {
        ReferencePatIndex::new(((self.value >> 3) & 0b11) as u8)
    }
    #[inline]
    pub const fn target_frame(&self) -> Frame<Frame4KiB> {
        Frame::containing(PhysAddr::new_panic(
            (self.value & (Frame4KiB::MASK & PhysAddr::MASK) as u64) as usize,
        ))
    }
}

impl PagingRootEntry {
    pub fn target<'a>(&self) -> &'a [PagingReferenceEntry<Page512GiB>; 512] {
        unsafe {
            // SAFETY: Safe because this type is !Send & !Sync
            self.target_frame()
                .to_virt::<Page4KiB>()
                .boundary()
                .to_ref()
        }
    }
    pub fn target_mut<'a>(&self) -> &'a mut [PagingReferenceEntry<Page512GiB>; 512] {
        unsafe {
            // SAFETY: Safe because this type is !Send & !Sync
            self.target_frame()
                .to_virt::<Page4KiB>()
                .boundary()
                .to_mut()
        }
    }
}

impl PagingRootEntry {
    #[inline]
    pub fn current() -> Self {
        let value: u64;
        unsafe {
            asm!(
                "mov {value}, cr3",
                value = out(reg) value,
            );
        }
        Self { value }
    }
    #[inline]
    pub fn load(&self) {
        unsafe {
            asm!(
                "mov cr3, {value}",
                value = in(reg) self.value,
            );
        }
    }
}

impl Deref for PagingRootEntry {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
