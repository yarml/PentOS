mod fb_impl;

use {
    crate::{bootinfo, dev::framebuffer::fb_impl::Framebuffer},
    spinlocks::{
        mutex::{SpinMutex, SpinMutexGuard},
        once::SpinOnce,
    },
};

static MAIN_FRAMEBUFFER: SpinOnce<SpinMutex<Framebuffer>> = SpinOnce::new();

pub(crate) fn init() {
    let bootinfo = bootinfo::bootinfo();
    MAIN_FRAMEBUFFER.init(|| {
        SpinMutex::new(unsafe {
            // SAFETY: FramebufferInfo from bootinfo should be valid
            Framebuffer::from_info(&bootinfo.framebuffer)
        })
    });
}

pub fn lock() -> SpinMutexGuard<'static, Framebuffer> {
    MAIN_FRAMEBUFFER
        .poll()
        .expect("main framebuffer not initialized")
        .lock()
}
