use core::arch::asm;
use core::marker::PhantomData;

pub struct Port<N> {
    port: u16,
    _phantom: PhantomData<N>,
}

impl<N> Port<N> {
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            _phantom: PhantomData,
        }
    }
}

impl Port<u8> {
    /// # Safety
    /// IO port reading could have side effects
    pub unsafe fn read(&self) -> u8 {
        let mut value;
        unsafe {
            asm! {
                "in al, dx",
                in("dx") self.port,
                out("al") value,
            };
        }
        value
    }
    /// # Safety
    /// IO writing reading could have side effects
    pub unsafe fn write(&self, value: u8) {
        unsafe {
            asm! {
                "out dx, al",
                in("dx") self.port,
                in("al") value,
            }
        }
    }
}

impl Port<u16> {
    /// # Safety
    /// IO port reading could have side effects
    pub unsafe fn read(&self) -> u16 {
        let mut value;
        unsafe {
            asm! {
                "in ax, dx",
                in("dx") self.port,
                out("ax") value,
            };
        }
        value
    }
    /// # Safety
    /// IO port writing could have side effects
    pub unsafe fn write(&self, value: u16) {
        unsafe {
            asm! {
                "out dx, ax",
                in("dx") self.port,
                in("ax") value,
            }
        }
    }
}

impl Port<u32> {
    /// # Safety
    /// IO port reading could have side effects
    pub unsafe fn read(&self) -> u32 {
        let mut value;
        unsafe {
            asm! {
                "in eax, dx",
                in("dx") self.port,
                out("eax") value,
            };
        }
        value
    }
    /// # Safety
    /// IO port writing could have side effects
    pub unsafe fn write(&self, value: u32) {
        unsafe {
            asm! {
                "out dx, eax",
                in("dx") self.port,
                in("eax") value,
            }
        }
    }
}

pub fn wait() {
    // https://wiki.osdev.org/Inline_Assembly/Examples#IO_WAIT
    let tmp = Port::<u8>::new(0x80);
    unsafe {
        // # Safety
        // This should be an unused port?
        tmp.write(0)
    };
}
