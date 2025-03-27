use core::marker::PhantomData;

use crate::mem::page::size::PageSize;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PagingAbsentEntry<PS: PageSize> {
    value: u64,
    _phantom: PhantomData<PS>,
}

impl<PS: PageSize> PagingAbsentEntry<PS> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            value: 0,
            _phantom: PhantomData,
        }
    }

    pub const fn from_inner(value: u64) -> Self {
        assert!(value & 1 == 0);
        Self {
            value: value,
            _phantom: PhantomData,
        }
    }
}

impl<PS: PageSize> PagingAbsentEntry<PS> {}
