pub mod framebuffer;
pub mod ps2;
pub mod timer;
pub mod pci;

pub(crate) fn init() {
    framebuffer::init();
    ps2::init();
    pci::init();
}
