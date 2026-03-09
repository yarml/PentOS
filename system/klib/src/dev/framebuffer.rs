mod fb_impl;

use {
    crate::{bootinfo, dev::framebuffer::fb_impl::Framebuffer},
    spinlocks::{
        mutex::{Mutex, MutexGuard},
        once::Once,
    },
};

static MAIN_FRAMEBUFFER: Once<Mutex<Framebuffer>> = Once::new();

pub(crate) fn init() {
    let bootinfo = bootinfo::bootinfo();
    MAIN_FRAMEBUFFER.init(|| {
        Mutex::new(unsafe {
            // SAFETY: FramebufferInfo from bootinfo should be valid
            Framebuffer::from_info(&bootinfo.framebuffer)
        })
    });
}

pub fn lock() -> MutexGuard<'static, Framebuffer> {
    MAIN_FRAMEBUFFER
        .poll()
        .expect("main framebuffer not initialized")
        .lock()
}
