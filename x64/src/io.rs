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
    pub unsafe fn read(&mut self) -> u8 {
        let mut value;
        unsafe {
            asm! {
                "inb al, dx",
                in("dx") self.port,
                out("al") value,
            };
        }
        value
    }
    /// # Safety
    /// IO writing reading could have side effects
    pub unsafe fn write(&mut self, value: u8) {
        unsafe {
            asm! {
                "outb al, dx",
                in("dx") self.port,
                in("al") value,
            }
        }
    }
}

impl Port<u16> {
    /// # Safety
    /// IO port reading could have side effects
    pub unsafe fn read(&mut self) -> u16 {
        let mut value;
        unsafe {
            asm! {
                "inb ax, dx",
                in("dx") self.port,
                out("ax") value,
            };
        }
        value
    }
    /// # Safety
    /// IO port writing could have side effects
    pub unsafe fn write(&mut self, value: u16) {
        unsafe {
            asm! {
                "outb ax, dx",
                in("dx") self.port,
                in("ax") value,
            }
        }
    }
}

impl Port<u32> {
    /// # Safety
    /// IO port reading could have side effects
    pub unsafe fn read(&mut self) -> u32 {
        let mut value;
        unsafe {
            asm! {
                "inb eax, dx",
                in("dx") self.port,
                out("eax") value,
            };
        }
        value
    }
    /// # Safety
    /// IO port writing could have side effects
    pub unsafe fn write(&mut self, value: u32) {
        unsafe {
            asm! {
                "outb al, dx",
                in("dx") self.port,
                in("eax") value,
            }
        }
    }
}

pub fn wait() {
    // https://wiki.osdev.org/Inline_Assembly/Examples#IO_WAIT
    let mut tmp = Port::<u8>::new(0x80);
    unsafe {
        // # Safety
        // This should be an unused port?
        tmp.write(0)
    };
}
