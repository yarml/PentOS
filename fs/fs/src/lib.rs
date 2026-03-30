#![no_std]

use alloc::string::String;

extern crate alloc;

#[derive(Debug, Clone)]
pub struct FilesystemInfo {
    pub label: Option<String>,
    pub read_only: bool,
}
