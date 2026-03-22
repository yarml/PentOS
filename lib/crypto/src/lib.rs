#![no_std]

mod crc32;

pub use crc32::{crc32, crc32_continue, crc32_zdata};
