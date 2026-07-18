#![no_std]

pub mod sleep;
pub mod suspend;

extern crate alloc;

use {
    core::sync::atomic::{AtomicUsize, Ordering},
    klib::{dev::driver, interrupts::lapic, task},
};

static TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

#[driver]
fn init() {
    lapic::attach_tick_listener(on_tick);
}

pub fn get_timestamp() -> usize {
    TIMESTAMP.load(Ordering::Relaxed)
}

fn on_tick() {
    TIMESTAMP.fetch_add(1, Ordering::Relaxed);
    task::spawn(wake_all());
}

async fn wake_all() {
    sleep::wake().await;
    suspend::wake().await;
}
