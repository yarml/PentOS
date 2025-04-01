use super::RawMsr;

const MSR: u32 = 0xC000_0080;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Efer {
    raw: RawMsr,
}

impl Efer {
    /// LME, and LMA always set
    pub fn new() -> Self {
        Self {
            raw: RawMsr::new(0x50),
        }
    }

    pub fn read() -> Self {
        let mut raw = RawMsr::read(MSR);
        *raw |= 0x50; // Set LME and LMA bits
        Self { raw }
    }

    pub fn write(&self) {
        self.raw.write(MSR);
    }
}

impl Efer {
    pub fn syscall(&mut self, val: bool) -> &mut Self {
        if val {
            *self.raw |= 1 << 0;
        } else {
            *self.raw &= !(1 << 0);
        }
        self
    }

    pub fn exec_disable(&mut self, val: bool) -> &mut Self {
        if val {
            *self.raw |= 1 << 11;
        } else {
            *self.raw &= !(1 << 11);
        }
        self
    }
}

impl Efer {
    pub fn is_syscall(&self) -> bool {
        *self.raw & (1 << 0) != 0
    }
    pub fn is_exec_disable(&self) -> bool {
        *self.raw & (1 << 11) != 0
    }
}
