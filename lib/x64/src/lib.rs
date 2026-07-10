#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(const_trait_impl)]

pub mod control;
pub mod interrupts;
pub mod io;
pub mod ioapic;
pub mod lapic;
pub mod mem;
pub mod msr;
pub mod prot;
