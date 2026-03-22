use core::fmt::{self, Debug, Display};

impl Guid {
    pub const EFI_SYSTEM: Guid = Guid::new(0xC12A7328_F81F_11D2_BA4B_00A0C93EC93B);
    pub const PENTOS_SYSTEM: Guid = Guid::new(0xBE179251_0C3E_49F7_9804_90395571005E);
    pub const LINUX_DATA: Guid = Guid::new(0x0FC63DAF_8483_4772_8E79_3D69D8477DE4);
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Guid(pub [u8; 16]);

impl Guid {
    pub const fn new(uuid: u128) -> Self {
        let d1 = ((uuid >> 96) as u32).to_le_bytes();
        let d2 = ((uuid >> 80) as u16).to_le_bytes();
        let d3 = ((uuid >> 64) as u16).to_le_bytes();
        let d4 = (uuid as u64).to_be_bytes();

        Self([
            d1[0], d1[1], d1[2], d1[3], d2[0], d2[1], d3[0], d3[1], d4[0], d4[1], d4[2], d4[3],
            d4[4], d4[5], d4[6], d4[7],
        ])
    }
}

// In a shared `utils` or dedicated `guid` crate
impl Guid {
    const fn new_v4_from_bytes(random_bytes: [u8; 16]) -> Self {
        let mut uuid = u128::from_be_bytes(random_bytes);
        uuid = (uuid & !(0xf << 76)) | (0x4 << 76);
        uuid = (uuid & !(0x3 << 62)) | (0x2 << 62);

        Self::new(uuid)
    }

    #[cfg(target_os = "none")]
    pub fn gen_v4() -> Self {
        use core::arch::asm;
        let mut bytes = [0u8; 16];
        for chunk in bytes.chunks_mut(4) {
            let val: u32;
            loop {
                let success: u8;
                unsafe {
                    asm!(
                        "rdrand {val:e}",
                        "setc {success}",
                        val = out(reg) val,
                        success = out(reg_byte) success,
                    );
                }
                if success != 0 {
                    break;
                }
            }
            chunk.copy_from_slice(&val.to_ne_bytes());
        }
        Self::new_v4_from_bytes(bytes)
    }

    #[cfg(not(target_os = "none"))]
    pub fn gen_v4() -> Self {
        use getrandom;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).expect("getrandom failed");
        Self::new_v4_from_bytes(bytes)
    }
}

impl Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d1 = u32::from_le_bytes([self.0[0], self.0[1], self.0[2], self.0[3]]);
        let d2 = u16::from_le_bytes([self.0[4], self.0[5]]);
        let d3 = u16::from_le_bytes([self.0[6], self.0[7]]);
        write!(
            f,
            "{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
            d1,
            d2,
            d3,
            self.0[8],
            self.0[9],
            self.0[10],
            self.0[11],
            self.0[12],
            self.0[13],
            self.0[14],
            self.0[15],
        )
    }
}

impl Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guid({})", self)
    }
}
