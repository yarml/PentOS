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
mod lapic_timer;
mod loader;
mod misc;
mod panic;
mod phys_mmap;
mod pic;
mod segmentation;
mod timers;
mod topology;
mod virt_mmap;
