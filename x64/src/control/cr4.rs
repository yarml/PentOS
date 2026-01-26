use core::arch::asm;

#[derive(Clone, Copy)]
pub struct CR4 {
    raw: u64,
}

impl CR4 {
    pub fn read() -> Self {
        let raw: u64;
        unsafe {
            // SAFETY: no issue
            asm!("mov {0}, cr4", out(reg) raw);
        }
        Self { raw }
    }

    /// # Safety
    /// Must guarentee that the new configuration will not cause an undefined behaviour
    /// in rust
    pub unsafe fn write(&self) {
        unsafe {
            // SAFETY: Guarenteed by caller
            asm!("mov cr4, {0}", in(reg) self.raw)
        }
    }
}

impl CR4 {
    pub const fn rawval(&self) -> u64 {
        self.raw
    }
}
