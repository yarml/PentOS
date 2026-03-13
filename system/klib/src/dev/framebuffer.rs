mod fb_impl;

use {
    crate::{
        bootinfo,
        dev::framebuffer::fb_impl::Framebuffer,
        sync::mutex::{AsyncMutex, AsyncMutexGuard},
    },
    spinlocks::once::SpinOnce,
};

static MAIN_FRAMEBUFFER: SpinOnce<AsyncMutex<Framebuffer>> = SpinOnce::new();

pub(crate) fn init() {
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
