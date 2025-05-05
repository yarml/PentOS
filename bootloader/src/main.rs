#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(cold_path)]

mod acpi;
mod allocator;
mod bootstage;
mod entry;
mod features;
mod framebuffer;
mod gdt;
mod infoarea;
mod kernel;
mod logger;
mod misc;
mod mp;
mod panic;
mod phys_mmap;
mod pic;
mod pit;
mod topology;
mod virt_mmap;
