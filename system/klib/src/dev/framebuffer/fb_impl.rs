use core::{arch::x86_64::*, slice};

use system::framebuffer::{FramebufferInfo, PixelColor, PixelMode};

#[derive(Clone, Copy)]
struct DirtyRect {
    x_min: usize,
    y_min: usize,
    x_max: usize,
    y_max: usize,
}

impl DirtyRect {
    const CLEAN: Self = Self {
        x_min: usize::MAX,
        y_min: usize::MAX,
        x_max: 0,
        y_max: 0,
    };

    #[inline(always)]
    fn is_clean(&self) -> bool {
        self.x_min >= self.x_max || self.y_min >= self.y_max
    }

    #[inline(always)]
    fn mark(&mut self, x: usize, y: usize) {
        if x < self.x_min {
            self.x_min = x;
        }
        if x + 1 > self.x_max {
            self.x_max = x + 1;
        }
        if y < self.y_min {
            self.y_min = y;
        }
        if y + 1 > self.y_max {
            self.y_max = y + 1;
        }
    }

    #[inline(always)]
    fn mark_rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if x < self.x_min {
            self.x_min = x;
        }
        if x + w > self.x_max {
            self.x_max = x + w;
        }
        if y < self.y_min {
            self.y_min = y;
        }
        if y + h > self.y_max {
            self.y_max = y + h;
        }
    }
}

pub struct Framebuffer {
    fb: *mut u32,
    buffer: &'static mut [u32],
    width: usize,
    height: usize,
    stride: usize,
    mode: PixelMode,
    dirty: DirtyRect,
}

unsafe impl Send for Framebuffer {}

impl Framebuffer {
    /// # Safety
    /// FramebufferInfo's fbptr and bufferptr must be valid memory within kernel space.
    /// fb must be mapped WriteCombining, buffer must be mapped WriteBack.
    pub unsafe fn from_info(fbinfo: &FramebufferInfo) -> Self {
        let buffer = unsafe { slice::from_raw_parts_mut(fbinfo.bufferptr, fbinfo.len) };
        Self {
            fb: fbinfo.fbptr,
            buffer,
            width: fbinfo.width,
            height: fbinfo.height,
            stride: fbinfo.stride,
            mode: fbinfo.mode,
            dirty: DirtyRect::CLEAN,
        }
    }

    #[inline(always)]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline(always)]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: PixelColor) {
        debug_assert!(x < self.width && y < self.height);
        let idx = y * self.stride + x;
        self.buffer[idx] = color.encode(self.mode);
        self.dirty.mark(x, y);
    }

    pub fn draw_box(&mut self, x: usize, y: usize, w: usize, h: usize, color: PixelColor) {
        debug_assert!(x + w <= self.width && y + h <= self.height);
        let encoded = color.encode(self.mode);
        for row in y..y + h {
            let start = row * self.stride + x;
            self.buffer[start..start + w].fill(encoded);
        }
        self.dirty.mark_rect(x, y, w, h);
    }

    /// Flush only the dirty bounding rectangle from the WriteBack buffer into
    /// the WriteCombining framebuffer.
    ///
    /// Strategy for WB -> WC transfer:
    ///   1. Copy row-by-row so WC's write-combining buffers
    ///      can be filled and flushed as a unit.
    ///   2. Use non-temporal stores (MOVNTI) so the CPU never
    ///      allocates a cache line for the WC destination, avoiding cache
    ///      pollution and a pointless WB->WC snoop.
    ///   3. A single SFENCE after all stores ensures the write-combining buffers
    ///      are flushed to the display controller before we return.
    pub fn refresh(&mut self) {
        let dirty = self.dirty;
        if dirty.is_clean() {
            return;
        }
        self.dirty = DirtyRect::CLEAN;

        let x0 = dirty.x_min;
        let x1 = dirty.x_max;
        let row_pixels = x1 - x0;

        for row in dirty.y_min..dirty.y_max {
            let src_start = row * self.stride + x0;
            let dst_start = row * self.stride + x0;

            let src = &self.buffer[src_start..src_start + row_pixels];

            let dst = unsafe {
                // SAFETY: fb is valid WC memory covering at least stride * height u32s.
                slice::from_raw_parts_mut(self.fb.add(dst_start), row_pixels)
            };

            unsafe { copy_row_nt(src, dst) };
        }

        unsafe { _mm_sfence() };
    }

    /// Invalidate the entire screen so the next refresh() repaints everything.
    #[inline]
    pub fn mark_all_dirty(&mut self) {
        self.dirty = DirtyRect {
            x_min: 0,
            y_min: 0,
            x_max: self.width,
            y_max: self.height,
        };
    }
}

/// Copy one row of pixels from a WB source slice to a WC destination slice
/// using scalar non-temporal stores (MOVNTI).
#[inline]
unsafe fn copy_row_nt(src: &[u32], dst: &mut [u32]) {
    debug_assert_eq!(src.len(), dst.len());
    let src_ptr = src.as_ptr();
    let dst_ptr = dst.as_mut_ptr();
    for i in 0..src.len() {
        unsafe {
            _mm_stream_si32(dst_ptr.add(i) as *mut i32, *src_ptr.add(i) as i32);
        }
    }
}
