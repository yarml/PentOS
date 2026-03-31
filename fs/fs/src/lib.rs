#![no_std]

use core::pin::Pin;

use {
    alloc::{boxed::Box, string::String},
    io::IoResult,
};

extern crate alloc;

#[derive(Debug, Clone)]
pub struct FilesystemInfo {
    pub label: Option<String>,
    pub read_only: bool,
}

pub trait OpenFile {
    fn read<'a>(&'a self, buf: &'a mut [u8]) -> Pin<Box<dyn Future<Output = IoResult<usize>> + Send + 'a>>;
    fn write<'a>(&'a self, buf: &'a [u8]) -> Pin<Box<dyn Future<Output = IoResult<usize>> + Send + 'a>>;
}
