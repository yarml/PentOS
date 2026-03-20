#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphResult {
    Incomplete,
    Found(u16),
    /// fallback glyph index (U+FFFD or glyph 0 if U+FFFD is absent).
    Fallback(u16),
}

pub struct FontStateMachine {
    state: u16,
}

impl FontStateMachine {
    pub const fn new() -> Self {
        Self { state: 0 }
    }

    pub fn reset(&mut self) {
        self.state = 0;
    }
}

impl Default for FontStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

include!(concat!(env!("OUT_DIR"), "/state_machine.rs"));
include!(concat!(env!("OUT_DIR"), "/glyphs.rs"));
include!(concat!(env!("OUT_DIR"), "/feed.rs"));
