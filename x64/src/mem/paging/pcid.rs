use core::ops::Deref;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pcid {
    value: u16,
}

impl Pcid {
    #[inline]
    pub const fn new(value: u16) -> Self {
        assert!(value < 0x1000);
        Self { value }
    }
}

impl Pcid {
    #[inline]
    pub const fn unwrap(&self) -> u16 {
        self.value
    }
}

impl Deref for Pcid {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
