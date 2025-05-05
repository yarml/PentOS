use super::absent_entry::PagingAbsentEntry;
use super::pat::ReferencePatIndex;
use super::raw_entry::PagingRawEntry;
use crate::mem::addr::Address;
use crate::mem::addr::PhysAddr;
use crate::mem::frame::Frame;
use crate::mem::frame::size::Frame4KiB;
use crate::mem::frame::size::FrameSize;
use crate::mem::page::size::Page4KiB;
use crate::mem::page::size::PageSize;
use core::marker::PhantomData;
use core::ops::Deref;

/// Acts like a &mut PagingReferenceEntry
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PagingReferenceEntry<PS>
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    value: u64,
    _phantom: PhantomData<PS>,
}

impl<PS> PagingReferenceEntry<PS>
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    /// Defaults to nowrite, nouser, noexec, PAT = 0
    #[inline]
    pub const fn new(target_frame: Frame<Frame4KiB>) -> Self {
        Self {
            value: (1 << 63) | (1 << 0) | target_frame.boundary().as_u64(),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn from_inner(value: u64) -> Self {
        assert!(value == ((value | (1 << 0)) & !PS::USE_MAP_FLAG));
        Self {
            value,
            _phantom: PhantomData,
        }
    }
}

impl<PS> PagingReferenceEntry<PS>
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    #[inline]
    pub const fn nopresent(self) -> PagingAbsentEntry<PS> {
        PagingAbsentEntry::from_inner(self.value & !(1 << 0))
    }
    #[inline]
    pub const fn write(self) -> Self {
        Self {
            value: self.value | (1 << 1),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn nowrite(self) -> Self {
        Self {
            value: self.value & !(1 << 1),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn user(self) -> Self {
        Self {
            value: self.value | (1 << 2),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn nouser(self) -> Self {
        Self {
            value: self.value & !(1 << 2),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn exec(self) -> Self {
        Self {
            value: self.value & !(1 << 63),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn noexec(self) -> Self {
        Self {
            value: self.value | (1 << 63),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn with_pat_index(self, index: ReferencePatIndex) -> Self {
        Self {
            value: self.value & !(0b11 << 2) | index.pgentry_flags(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn with_target(self, target_frame: Frame<Frame4KiB>) -> Self {
        Self {
            value: (self.value & !(Frame4KiB::MASK & PhysAddr::MASK) as u64)
                | target_frame.boundary().as_u64(),
            _phantom: PhantomData,
        }
    }
}

impl<PS> PagingReferenceEntry<PS>
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    #[inline]
    pub const fn is_write(&self) -> bool {
        self.value & (1 << 1) != 0
    }
    #[inline]
    pub const fn is_user(&self) -> bool {
        self.value & (1 << 2) != 0
    }
    #[inline]
    pub const fn is_exec(&self) -> bool {
        self.value & (1 << 63) == 0
    }

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

    #[inline]
    pub const fn is_accessed(&self) -> bool {
        self.value & (1 << 5) != 0
    }
}

impl<PS> PagingReferenceEntry<PS>
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    #[inline]
    pub const fn as_raw(&mut self) -> &mut PagingRawEntry<PS> {
        unsafe { &mut *(self as *mut Self as *mut PagingRawEntry<PS>) }
    }

    #[inline]
    pub const fn to_raw(&self) -> PagingRawEntry<PS> {
        PagingRawEntry::new(self.value)
    }
}

impl<PS> PagingReferenceEntry<PS>
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    /// # Safety
    /// Must ensure that this entry is pointing to a valid sub table
    /// and that the memory location is not mutably aliased
    pub unsafe fn target<'a>(&self) -> &'a [PagingRawEntry<PS::ReferenceTarget>; 512]
    where
        PS::ReferenceTarget: PageSize,
    {
        unsafe {
            self.target_frame()
                .to_virt::<Page4KiB>()
                .boundary()
                .to_ref()
        }
    }

    /// # Safety
    /// Must ensure that this entry is pointing to a valid sub table
    /// and that the memory location is not mutably aliased
    pub unsafe fn target_mut<'a>(&self) -> &'a mut [PagingRawEntry<PS::ReferenceTarget>; 512]
    where
        PS::ReferenceTarget: PageSize,
    {
        unsafe {
            self.target_frame()
                .to_virt::<Page4KiB>()
                .boundary()
                .to_mut()
        }
    }
}

impl<PS> Deref for PagingReferenceEntry<PS>
where
    PS: PageSize,
    PS::ReferenceTarget: PageSize,
{
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
