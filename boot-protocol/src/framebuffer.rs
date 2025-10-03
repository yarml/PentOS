use core::slice;

#[repr(C)]
pub struct FramebufferInfo {
    pub fbptr: *mut u32,
    pub fblen: usize,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub bufferptr: *mut u32,
    pub bufferlen: usize,
}

impl FramebufferInfo {
    pub fn fb(&mut self) -> &mut [u32] {
        unsafe { slice::from_raw_parts_mut(self.fbptr, self.fblen) }
    }

    pub fn buffer(&mut self) -> &mut [u32] {
        unsafe { slice::from_raw_parts_mut(self.bufferptr, self.bufferlen) }
    }
}
