#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PixelColor(pub u8, pub u8, pub u8);

#[derive(Clone, Copy)]
pub enum PixelMode {
    RgbRs,
    BgrRs,
}

impl PixelColor {
    pub fn encode(&self, mode: PixelMode) -> u32 {
        let (b0, b1, b2, b3) = match mode {
            PixelMode::RgbRs => (self.0, self.1, self.2, 0),
            PixelMode::BgrRs => (self.2, self.1, self.0, 0),
        };
        (b3 as u32) << 24 | (b2 as u32) << 16 | (b1 as u32) << 8 | (b0 as u32)
    }
}
