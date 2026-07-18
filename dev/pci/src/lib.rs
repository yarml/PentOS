#![no_std]
#![feature(const_trait_impl)]
#![feature(gen_blocks)]

pub mod adr;

#[cfg(feature = "driver")]
mod driver;

use {
    core::fmt::{self, Display, Formatter},
    x64::mem::addr::{Address, VirtAddr},
};

#[cfg(feature = "driver")]
use klib::dev::driver;

#[cfg(feature = "driver")]
#[driver]
pub fn init() {
    use klib::log::info;
    for (func_addr, info) in walk() {
        info!("{func_addr}: {info}");
    }
}

#[cfg(feature = "driver")]
pub use driver::*;

#[derive(Debug, Clone, Copy)]
pub struct CommonInfo {
    pub vendid: usize,
    pub devid: usize,
    pub class: usize,
    pub subclass: usize,
    pub prog_interface: usize,
    pub revision: usize,
    pub multifunction: bool,
    pub header_type: HeaderType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderType {
    GeneralDevice,
    PciBridge,
    CardBusBridge,
}

pub struct ConfigSpace {
    ptr: *mut u32,
}

impl ConfigSpace {
    /// # Safety
    /// `base` must point to a valid virtual address with Uncacheable mapping to a PCI function
    pub const unsafe fn new(base: VirtAddr) -> Self {
        Self {
            ptr: base.as_mut_ptr(),
        }
    }

    /// # Safety
    /// `reg` must be within boundaries of the PCI function configurationSpace
    pub unsafe fn read_reg(&self, reg: usize) -> u32 {
        unsafe { self.ptr.add(reg).read_volatile() }
    }

    /// Returns `None` if empty PCI slot, or slot contains invalid information
    pub fn read_info(&self) -> Option<CommonInfo> {
        let id = unsafe { self.read_reg(0) };
        let (vendid, devid) = ((id & 0xFFFF) as usize, (id >> 16) as usize);
        if vendid == 0xFFFF {
            return None;
        }

        let [class, subclass, interface, rev] = unsafe { self.read_reg(2) }.to_le_bytes();
        let [_, header_type, _, _] = unsafe { self.read_reg(3) }.to_le_bytes();

        let multifunction = header_type & 0x80 != 0;
        let header_type = HeaderType::try_from(header_type).ok()?;

        Some(CommonInfo {
            vendid,
            devid,
            class: class as usize,
            subclass: subclass as usize,
            prog_interface: interface as usize,
            revision: rev as usize,
            multifunction,
            header_type,
        })
    }
}

impl TryFrom<u8> for HeaderType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value & 0x7F {
            0x00 => Ok(Self::GeneralDevice),
            0x01 => Ok(Self::PciBridge),
            0x02 => Ok(Self::CardBusBridge),
            _ => Err(()),
        }
    }
}

impl Display for CommonInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vendid, self.devid)
    }
}
