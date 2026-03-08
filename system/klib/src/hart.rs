use {
    core::{cell::UnsafeCell, marker::PhantomData},
    system::hart::HartInfo,
};

pub use klib_macros::hart_local;

#[repr(transparent)]
pub struct HartLocal<T> {
    offset: *const u8,
    _phantom: PhantomData<T>,
}

/// # Safety
/// HartLocal uses kernel TLS loaded from GS offset to access private data for each hart
unsafe impl<T> Sync for HartLocal<T> {}

impl<T> HartLocal<T> {
    /// # Safety
    /// This relies on rf being in .hart_local section, and that the linker script
    /// has the .hart_local loaded into ORIGIN=0
    /// This function should only be called through the #\[hart_local] attribute
    pub const unsafe fn new(rf: &T) -> Self {
        let offset = rf as *const T as *const u8;

        Self {
            offset,
            _phantom: PhantomData,
        }
    }
}

impl<T> HartLocal<T> {
    #[inline(always)]
    /// # Safety
    /// GS needs to point to hart local data structure
    unsafe fn get_ptr(&self) -> *const UnsafeCell<T> {
        let hart_info = HartInfo::get();
        let base = hart_info.tls_base;
        (base + self.offset as usize) as *const UnsafeCell<T>
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let ptr = unsafe {
            // SAFETY: If running in kernel mode, GS is set to hart local structure
            self.get_ptr()
        };
        unsafe {
            // SAFETY: Safe unless GS is not placed correctly, which shouldn't happen
            f(&*(&*ptr).get())
        }
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let ptr = unsafe {
            // SAFETY: If running in kernel mode, GS is set to hart local structure
            self.get_ptr()
        };
        unsafe {
            // SAFETY: Safe unless GS is not placed correctly, which shouldn't happen
            f(&mut *(&*ptr).get())
        }
    }
}
