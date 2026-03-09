#![no_std]
#![no_main]

use {
    klib::task::{self, sleep::sleep, suspend::suspend},
    log::debug,
};

klib::use_klib!(kmain);

// Currently kmain is just testing the async system

async fn kmain() {
    task::spawn(task1());
    suspend().await;
    task::spawn(task2());
}

async fn task1() {
    loop {
        debug!("Task 1");
        sleep(1000).await;
    }
}

async fn task2() {
    loop {
        debug!("Task 2");
        sleep(5000).await;
    }
}
