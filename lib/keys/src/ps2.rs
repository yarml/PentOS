use crate::KeyEvent;

pub const MAX_SEQUENCE: usize = 8;

pub struct StateMachine {
    state: usize,
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

include!(concat!(env!("OUT_DIR"), "/ps2_state_machine.rs"));
