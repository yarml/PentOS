#![no_std]
#![feature(cold_path)]
#![feature(abi_x86_interrupt)]

#[cfg(test)]
extern crate alloc;

pub mod framebuffer;
pub mod interrupts;
pub mod io;
pub mod lapic;
pub mod mem;
pub mod msr;
pub mod prot;
