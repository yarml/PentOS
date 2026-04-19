use {crate::status::Status, std::fmt::{Debug, Display}};

pub trait ResultExt<T> {
    fn or_fatal(self, message: impl Display) -> T;
}

impl<T, E: Debug> ResultExt<T> for Result<T, E> {
    fn or_fatal(self, message: impl Display) -> T {
        match self {
            Ok(v) => v,
            Err(e) => {
                Status::error(format!("{message}: {e:?}"));
            }
        }
    }
}
