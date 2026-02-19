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
    /// PAE
    pub const fn new() -> Self {
        Self { raw: 0x00000020 } // PAE set
    }
}

impl CR4 {
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
    pub const fn global_pages(&mut self, global_pages: bool) -> &mut Self {
        if global_pages {
            self.raw |= 1 << 7;
        } else {
            self.raw &= !(1 << 7);
        }
        self
    }

    pub const fn debug_extensions(&mut self, debug_extensions: bool) -> &mut Self {
        if debug_extensions {
            self.raw |= 1 << 3;
        } else {
            self.raw &= !(1 << 3);
        }
        self
    }
}

impl CR4 {
    pub const fn rawval(&self) -> u64 {
        self.raw
    }
}

impl Default for CR4 {
    fn default() -> Self {
        Self::new()
    }
}
