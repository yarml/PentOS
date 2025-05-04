#![no_std]
#![no_main]
#![allow(dead_code)]

mod acpi;
mod allocator;
mod bootstage;
mod entry;
mod features;
mod framebuffer;
mod infoarea;
mod kernel;
mod logger;
mod misc;
mod panic;
mod phys_mmap;
mod pic;
mod pit;
mod topology;
mod virt_mmap;
