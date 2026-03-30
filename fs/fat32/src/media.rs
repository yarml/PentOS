
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Fixed,
    Removable,
}

impl MediaType {
    pub const fn code(&self) -> u8 {
        match self {
            MediaType::Fixed => 0xF8,
            MediaType::Removable => 0xF0,
        }
    }
}