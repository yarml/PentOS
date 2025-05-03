#![no_std]
#![feature(cold_path)]

#[cfg(test)]
extern crate alloc;

pub mod framebuffer;
pub mod lapic;
pub mod mem;
pub mod msr;
pub mod prot;
