use crate::mem::page::size::PageSize;
use core::ops::Deref;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PatIndex {
    value: u8,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReferencePatIndex {
    value: u8,
}

impl PatIndex {
    pub const fn new(value: u8) -> Self {
        assert!(value < 8);
        Self { value }
    }
}
impl ReferencePatIndex {
    pub const fn new(value: u8) -> Self {
        assert!(value < 4);
        Self { value }
    }
}

impl PatIndex {
    pub const fn pgentry_flags<PS: PageSize>(&self) -> u64 {
        (self.value as u64 & 0b11) << 3 | (self.value as u64 & 0b100 >> 2) << PS::PAT_INDEX
    }
}
impl ReferencePatIndex {
    pub const fn pgentry_flags(&self) -> u64 {
        (self.value as u64) << 3
    }
}

impl Deref for PatIndex {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl Deref for ReferencePatIndex {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
