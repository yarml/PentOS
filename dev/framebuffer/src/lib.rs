#![no_std]

mod fb_impl;

use {
    klib::{bootinfo, dev::driver},
    spinlocks::once::SpinOnce,
    sync::{AsyncMutex, AsyncMutexGuard},
};

pub use fb_impl::Framebuffer;

static MAIN_FRAMEBUFFER: SpinOnce<AsyncMutex<Framebuffer>> = SpinOnce::new();

#[driver]
fn init() {
    let bootinfo = bootinfo::bootinfo();
    MAIN_FRAMEBUFFER.init(|| {
        AsyncMutex::new(unsafe {
            // SAFETY: FramebufferInfo from bootinfo should be valid
            Framebuffer::from_info(&bootinfo.framebuffer)
        })
    });
}

pub async fn lock() -> AsyncMutexGuard<'static, Framebuffer> {
    MAIN_FRAMEBUFFER
        .poll()
        .expect("main framebuffer not initialized")
        .lock()
        .await
}
