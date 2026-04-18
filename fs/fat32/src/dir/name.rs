use {
    alloc::string::String,
    io::{IoError, IoResult},
};

/// Case insensitive name key. We store the uppercase form so lookups
/// match FAT semantics.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UpperName(String);

impl UpperName {
    pub fn from_str(s: &str) -> Self {
        UpperName(s.chars().map(|c| c.to_ascii_uppercase()).collect())
    }
}

pub fn validate_name(name: &str) -> IoResult<()> {
    if name.is_empty() || name == "." || name == ".." {
        return Err(IoError::InvalidInput);
    }
    if name.encode_utf16().count() > 255 {
        return Err(IoError::InvalidInput);
    }
    Ok(())
}
