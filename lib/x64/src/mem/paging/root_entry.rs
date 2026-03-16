use {
    super::{pat::ReferencePatIndex, pcid::Pcid},
    crate::mem::{
        addr::{Address, PhysAddr},
        frame::{
            Frame,
            size::{Frame4KiB, FrameSize},
        },
        page::size::{Page4KiB, Page512GiB},
        paging::PagingRawEntry,
    },
    core::{arch::asm, ops::Deref},
};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PagingRootEntry {
    value: u64,
}

impl PagingRootEntry {
    #[inline(always)]
    pub const fn new(target_frame: Frame<Frame4KiB>) -> Self {
        Self {
            value: target_frame.boundary().as_u64(),
        }
    }
}

impl PagingRootEntry {
    #[inline(always)]
    pub const fn with_pat_index(self, index: ReferencePatIndex) -> Self {
        Self {
            value: self.value & !(0b11 << 2) | index.pgentry_flags(),
        }
    }
    #[inline(always)]
    pub const fn with_target(self, target_frame: Frame<Frame4KiB>) -> Self {
        Self {
            value: (self.value & !(Frame4KiB::MASK & PhysAddr::MASK) as u64)
                | target_frame.boundary().as_u64(),
        }
    }
    #[inline(always)]
    pub const fn with_pcid(self, pcid: Pcid) -> Self {
        Self {
            value: self.value & !(0xFFF) | (pcid.unwrap() as u64 & 0xFFF),
        }
    }
}

impl PagingRootEntry {
    #[inline(always)]
    pub const fn pat_index(&self) -> ReferencePatIndex {
        ReferencePatIndex::new(((self.value >> 3) & 0b11) as u8)
    }
    #[inline(always)]
    pub const fn target_frame(&self) -> Frame<Frame4KiB> {
        Frame::containing(PhysAddr::new_panic(
            (self.value & (Frame4KiB::MASK & PhysAddr::MASK) as u64) as usize,
        ))
    }
}

impl PagingRootEntry {
    /// # Safety
    /// Must ensure that this entry is pointing to a valid sub table
    /// and that the memory location is not mutably aliased
    pub unsafe fn target<'a>(&self) -> &'a [PagingRawEntry<Page512GiB>; 512] {
        unsafe {
            // SAFETY: ensured by caller
            self.target_frame()
                .to_virt::<Page4KiB>()
                .boundary()
                .to_ref()
        }
    }
    /// # Safety
    /// Must ensure that this entry is pointing to a valid sub table
    /// and that the memory location is not aliased
    pub unsafe fn target_mut<'a>(&self) -> &'a mut [PagingRawEntry<Page512GiB>; 512] {
        unsafe {
            // SAFETY: ensured by caller
            self.target_frame()
                .to_virt::<Page4KiB>()
                .boundary()
                .to_mut()
        }
    }
}

impl PagingRootEntry {
    #[inline(always)]
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
    #[inline(always)]
    pub fn load(&self) {
        unsafe {
            asm!(
                "mov cr3, {value}",
                value = in(reg) self.value,
            );
        }
    }
    #[inline(always)]
    pub const fn rawval(&self) -> u64 {
        self.value
    }
}

impl Deref for PagingRootEntry {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
