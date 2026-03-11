#![no_std]

use utils::collections::smallvec::SmallVec;

pub const MAX_SEQUENCE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Simple { scancode: u8 },
    Extended { scancode: u8 },
    PrintScreen,
    Pause,
}

impl Key {
    pub const fn simple(scancode: u8) -> Self {
        Self::Simple { scancode }
    }
    pub const fn extended(scancode: u8) -> Self {
        Self::Extended { scancode }
    }

    pub const fn print_screen() -> Self {
        Self::PrintScreen
    }
    pub const fn pause() -> Self {
        Self::Pause
    }
}

impl Key {
    pub fn press_sequence(&self) -> SmallVec<u8, MAX_SEQUENCE> {
        match self {
            Key::Simple { scancode } => SmallVec::from([*scancode]),
            Key::Extended { scancode } => SmallVec::from([0xE0, *scancode]),
            Key::PrintScreen => SmallVec::from([0xE0, 0x12, 0xE0, 0x7C]),
            Key::Pause => SmallVec::from([0xE1, 0x14, 0x77, 0xE1, 0xF0, 0x14, 0xF0, 0x77]),
        }
    }
    pub fn release_sequence(&self) -> Option<SmallVec<u8, MAX_SEQUENCE>> {
        match self {
            Key::Simple { scancode } => Some(SmallVec::from([0xF0, *scancode])),
            Key::Extended { scancode } => Some(SmallVec::from([0xE0, 0xF0, *scancode])),
            Key::PrintScreen => Some(SmallVec::from([0xE0, 0xF0, 0x7C, 0xE0, 0xF0, 0x12])),
            Key::Pause => None,
        }
    }

    pub fn can_release(&self) -> bool {
        self.release_sequence().is_some()
    }
}

pub struct StateMachine {
    state: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedResult {
    Incomplete,
    Invalid,
    Output(KeyEvent),
}

impl StateMachine {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self { state: 0 }
    }

    pub fn feed(&mut self, byte: u8) -> FeedResult {
        match STATE_TRANSITIONS[self.state][byte as usize] {
            None => {
                self.state = 0;
                FeedResult::Invalid
            }
            Some(next) => {
                self.state = next as usize;
                match STATE_OUTPUTS[self.state] {
                    Some(event) => {
                        self.state = 0;
                        FeedResult::Output(event)
                    }
                    None => FeedResult::Incomplete,
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = 0;
    }
}

include!(concat!(env!("OUT_DIR"), "/keys.rs"));
include!(concat!(env!("OUT_DIR"), "/state_machine.rs"));
