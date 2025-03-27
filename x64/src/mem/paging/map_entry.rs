use super::absent_entry::PagingAbsentEntry;
use super::pat::PatIndex;
use super::pk::ProtectionKey;
use crate::mem::addr::PhysAddr;
use crate::mem::frame::Frame;
use crate::mem::frame::size::FrameSize;
use crate::mem::page::size::PageSize;
use core::marker::PhantomData;
use core::ops::Deref;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PagingMapEntry<PS: PageSize> {
    value: u64,
    _phantom: PhantomData<PS>,
}

impl<PS: PageSize> PagingMapEntry<PS> {
    #[inline]
    /// Defaults to nowrite, nouser, noexec, noglobal, PAT = 0, PK = 0
    pub const fn new(target: Frame<PS::PhysicalPageSize>) -> Self {
        Self {
            value: (1 << 63) | (1 << 0) | PS::USE_MAP_FLAG | target.boundary().as_u64(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn from_inner(value: u64) -> Self {
        assert!(value == value | (1 << 0) | PS::USE_MAP_FLAG);
        Self {
            value,
            _phantom: PhantomData,
        }
    }
}

impl<PS: PageSize> PagingMapEntry<PS> {
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
    pub const fn global(self) -> Self {
        Self {
            value: self.value | (1 << 8),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn with_pat_index(self, index: PatIndex) -> Self {
        Self {
            value: self.value & !PS::PAT_MASK | index.pgentry_flags::<PS>(),
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub const fn with_pk(self, pk: ProtectionKey) -> Self {
        Self {
            value: self.value & !(0b1111 << 59) | pk.pgentry_flags(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn with_target(self, target_frame: Frame<PS::PhysicalPageSize>) -> Self {
        Self {
            value: self.value & !(PS::PhysicalPageSize::MASK & PhysAddr::MASK) as u64
                | target_frame.boundary().as_u64(),
            _phantom: PhantomData,
        }
    }
}

impl<PS: PageSize> PagingMapEntry<PS> {
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
        self.value & (1 << 63) != 0
    }

    #[inline]
    pub const fn pat_index(&self) -> PatIndex {
        let lower_bits = ((self.value >> 2) & 0b11) as u8;
        let upper_bit = ((self.value >> PS::PAT_INDEX) & 0b1) as u8;
        PatIndex::new(lower_bits | upper_bit << 2)
    }
    #[inline]
    pub const fn pk(&self) -> ProtectionKey {
        ProtectionKey::new(((self.value >> 59) & 0b1111) as u8)
    }

    #[inline]
    pub const fn is_accessed(&self) -> bool {
        self.value & (1 << 5) != 0
    }
    #[inline]
    pub const fn is_dirty(&self) -> bool {
        self.value & (1 << 6) != 0
    }
    #[inline]
    pub const fn is_global(&self) -> bool {
        self.value & (1 << 8) != 0
    }
}

impl<PS: PageSize> Deref for PagingMapEntry<PS> {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
