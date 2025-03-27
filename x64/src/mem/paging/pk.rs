use core::ops::Deref;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProtectionKey {
    value: u8,
}

impl ProtectionKey {
    #[inline]
    pub const fn new(value: u8) -> Self {
        assert!(value < 16);
        Self { value }
    }
}

impl ProtectionKey {
    #[inline]
    pub const fn pgentry_flags(self) -> u64 {
        (self.value as u64) << 59
    }
}

impl Deref for ProtectionKey {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
