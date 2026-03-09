#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct FeatureSet {
    pub vendor: Vendor,
    pub shadow_stack: bool,
    pub pk_user: bool,
    pub pk_super: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Vendor {
    GenuineIntel,
    AuthenticAMD,
    Other,
}

impl From<[u8; 12]> for Vendor {
    fn from(value: [u8; 12]) -> Self {
        match &value {
            b"GenuineIntel" => Self::GenuineIntel,
            b"AuthenticAMD" => Self::AuthenticAMD,
            _ => Self::Other,
        }
    }
}
