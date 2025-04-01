#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FeatureSet {
    pub vendor: Vendor,
    pub context_id: bool,
    pub inv_context_id: bool,
    pub shadow_stack: bool,
    pub pk_user: bool,
    pub pk_super: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    GenuineIntel,
    AuthenticAMD,
}

impl TryFrom<[u8; 12]> for Vendor {
    type Error = [u8; 12];

    fn try_from(value: [u8; 12]) -> Result<Self, Self::Error> {
        match &value {
            b"GenuineIntel" => Ok(Self::GenuineIntel),
            b"AuthenticAMD" => Ok(Self::AuthenticAMD),
            _ => Err(value),
        }
    }
}
