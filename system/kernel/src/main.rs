#![no_std]
#![no_main]

use {crate::version::VERSION, log::info};

mod version;

klib::use_klib!(kmain);

async fn kmain() {
    info!("PentOS v{VERSION}");
}
