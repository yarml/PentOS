#![no_std]
#![feature(generic_arg_infer)]
#![feature(unsafe_cell_access)]
#![feature(maybe_uninit_slice)]
#![feature(const_trait_impl)]

use boot_protocol::BootInfo;

#[cfg(test)]
extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod phys;

pub fn init(bootinfo: &BootInfo) {
    phys::init(&bootinfo.mmap[..bootinfo.mmap_len]);
}

