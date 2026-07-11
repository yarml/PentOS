#![no_std]
#![feature(const_trait_impl)]

pub mod adr;

#[derive(Debug, Clone, Copy)]
pub struct CommonInfo {
    pub vendor: usize,
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
