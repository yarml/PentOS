#![no_std]
#![allow(incomplete_features)] // FIXME: I don't want to resort to this
#![feature(cold_path)]
#![feature(abi_x86_interrupt)]
#![feature(const_trait_impl)]
// We going really hardcore with this one
#![feature(non_lifetime_binders)]

#[cfg(test)]
extern crate alloc;

pub mod framebuffer;
pub mod interrupts;
pub mod io;
pub mod lapic;
pub mod mem;
pub mod msr;
pub mod prot;
