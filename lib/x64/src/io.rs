use core::{arch::asm, marker::PhantomData};

pub struct Port<N> {
    port: u16,
    _phantom: PhantomData<N>,
}

impl<N> Port<N> {
    /// # Safety
    /// Must ensure no other instance of Port uses this port
    /// Port effectively acts as &mut T, even for read operations, since those
    /// can have side effects
    ///
    /// Port also should not be 0x80 as that is used within the io::wait implementation only
    pub const unsafe fn new(port: u16) -> Self {
        Self {
            port,
            _phantom: PhantomData,
        }
    }
}

impl Port<u8> {
    pub fn read(&mut self) -> u8 {
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
    pub fn write(&mut self, value: u8) {
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
    pub fn read(&mut self) -> u16 {
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
    pub fn write(&mut self, value: u16) {
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
    pub fn read(&mut self) -> u32 {
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
    pub fn write(&mut self, value: u32) {
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
    let mut tmp = unsafe {
        // SAFETY: we get special treatment to use port 0x80
        Port::<u8>::new(0x80)
    };

    tmp.write(0);
}
