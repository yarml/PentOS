use crate::kmain;

/// # Safety
/// Should be called by the bootloader after it has finished initializing everything
#[unsafe(no_mangle)]
unsafe extern "sysv64" fn init(is_bsp: bool) -> ! {
    unsafe {
        // SAFETY: Guarenteed by bootloader
        klib::init(is_bsp, kmain)
    }
}
