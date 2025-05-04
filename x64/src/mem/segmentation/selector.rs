use core::ops::Deref;

use crate::prot::PrivilegeLevel;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SegmentSelector {
    inner: u16,
}

impl SegmentSelector {
    pub const fn new(index: u16, rpl: PrivilegeLevel) -> Self {
        Self {
            inner: index << 3 | rpl as u16,
        }
    }
}

impl SegmentSelector {
    pub const fn get(&self) -> u16 {
        self.inner
    }
}

impl Deref for SegmentSelector {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
