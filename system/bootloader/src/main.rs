#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(const_trait_impl)]

mod acpi;
mod allocator;
mod bootstage;
mod entry;
mod features;
mod framebuffer;
mod hart;
mod kernel;
mod loader;
mod misc;
mod panic;
mod phys_mmap;
mod pic;
mod pit;
mod topology;
mod virt_mmap;
mod segmentation;
