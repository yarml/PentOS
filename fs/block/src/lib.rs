#![no_std]

use io::IoResult;

// The fundamental abstraction: something that reads and writes
// fixed-size sectors at arbitrary LBAs.
pub trait BlockDevice {
    /// Logical block size in bytes. Must be a power of 2 & >= 512.
    fn sector_size(&self) -> usize;

    /// Total number of addressable sectors on this device.
    fn sector_count(&self) -> u64;

    /// Read exactly one sector at `lba` into `buf`.
    /// `buf.len()` must equal `self.sector_size()`.
    fn read_sector(&mut self, lba: u64, buf: &mut [u8]) -> IoResult<()>;

    /// Write exactly one sector at `lba` from `buf`.
    /// `buf.len()` must equal `self.sector_size()`.
    fn write_sector(&mut self, lba: u64, buf: &[u8]) -> IoResult<()>;
}
