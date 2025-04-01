#[repr(C)]
pub struct FramebufferInfo {
    pub fb: &'static mut [u32],
    pub width: usize,
    pub height: usize,
    pub line_stride: usize,
    pub buffer: &'static mut [u32],
}
