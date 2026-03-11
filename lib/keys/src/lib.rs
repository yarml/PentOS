#![no_std]

pub const MAX_SEQUENCE: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Key {
    pub id: usize,
}

impl Key {
    const fn of_id(id: usize) -> Self {
        assert!(id < KEYS_COUNT);
        Self { id }
    }
}

pub struct StateMachine {
    state: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
    Tap(Key), // Keys which only have a press, but don't have a release emit this instead
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedResult {
    Incomplete,
    Invalid,
    Output(KeyEvent),
}

impl KeyEvent {
    pub fn key(&self) -> Key {
        match *self {
            KeyEvent::Pressed(k) => k,
            KeyEvent::Released(k) => k,
            KeyEvent::Tap(k) => k,
        }
    }

    pub fn is_pressed(&self) -> bool {
        matches!(*self, KeyEvent::Pressed(_))
    }

    pub fn is_released(&self) -> bool {
        matches!(*self, KeyEvent::Released(_))
    }

    pub fn is_tap(&self) -> bool {
        matches!(*self, KeyEvent::Tap(_))
    }
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
