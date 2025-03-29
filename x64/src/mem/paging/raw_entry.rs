use super::PagingMapEntry;
use super::PagingReferenceEntry;
use super::absent_entry::PagingAbsentEntry;
use crate::mem::frame::size::FrameSize;
use crate::mem::page::size::PageSize;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ops::DerefMut;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PagingRawEntry<PS: PageSize> {
    value: u64,
    _phantom: PhantomData<PS>,
}

impl<PS: PageSize> PagingRawEntry<PS> {
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }
}

impl<PS: PageSize> PagingRawEntry<PS> {
    #[inline]
    pub const fn as_absent(self) -> Option<PagingAbsentEntry<PS>> {
        if self.value & 1 == 0 {
            Some(PagingAbsentEntry::from_inner(self.value))
        } else {
            None
        }
    }
    #[inline]
    pub const fn as_map(self) -> Option<PagingMapEntry<PS>>
    where
        PS::PhysicalPageSize: FrameSize,
    {
        if self.value & 1 != 0 && self.value & PS::USE_MAP_FLAG != 0 {
            Some(PagingMapEntry::from_inner(self.value))
        } else {
            None
        }
    }
    #[inline]
    pub const fn as_reference(self) -> Option<PagingReferenceEntry<PS>>
    where
        PS::ReferenceTarget: PageSize,
    {
        if self.value & 1 != 0 && self.value & PS::USE_MAP_FLAG == 0 {
            Some(PagingReferenceEntry::from_inner(self.value))
        } else {
            None
        }
    }
}

impl<PS: PageSize> Deref for PagingRawEntry<PS> {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<PS: PageSize> DerefMut for PagingRawEntry<PS> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
