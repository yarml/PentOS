use core::arch::asm;

#[derive(Clone, Copy)]
pub struct CR0 {
    raw: u64,
}

impl CR0 {
    pub fn read() -> Self {
        let raw: u64;
        unsafe {
            // SAFETY: no issue
            asm!("mov {0}, cr0", out(reg) raw);
        }
        Self { raw }
    }
    pub const fn new() -> Self {
        Self { raw: 0x80000011 } // PG, ET, and PE always enabled
    }
}

impl CR0 {
    /// # Safety
    /// Must guarentee that the new configuration will not cause an undefined behaviour
    /// in rust
    pub unsafe fn write(&self) {
        unsafe {
            // SAFETY: Guarenteed by caller
            asm!("mov cr0, {0}", in(reg) self.raw)
        }
    }
}

impl CR0 {
    pub const fn write_protect(&mut self, write_protect: bool) -> &mut Self {
        if write_protect {
            self.raw |= 1 << 16;
        } else {
            self.raw &= !(1 << 16);
        }
        self
    }

    pub const fn numeric_error(&mut self, numeric_error: bool) -> &mut Self {
        if numeric_error {
            self.raw |= 1 << 5;
        } else {
            self.raw &= !(1 << 5);
        }
        self
    }
}

impl CR0 {
    pub const fn rawval(&self) -> u64 {
        self.raw
    }
}

impl Default for CR0 {
    fn default() -> Self {
        Self::new()
    }
}
