pub mod framebuffer;
pub mod ps2;
pub mod timer;

pub(crate) fn init() {
    framebuffer::init();
    ps2::init();
}
