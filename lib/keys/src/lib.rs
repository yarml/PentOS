#![no_std]

pub mod ps2;
pub mod layouts;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Pressed(Key),
    Repeated(Key),
    Released(Key),
    Tap(Key), // Keys which only have a press, but don't have a release emit this instead
}

impl KeyEvent {
    pub fn key(&self) -> Key {
        match *self {
            KeyEvent::Pressed(k) => k,
            KeyEvent::Repeated(k) => k,
            KeyEvent::Released(k) => k,
            KeyEvent::Tap(k) => k,
        }
    }

    pub fn is_pressed(&self) -> bool {
        matches!(*self, KeyEvent::Pressed(_) | KeyEvent::Repeated(_))
    }

    pub fn is_released(&self) -> bool {
        matches!(*self, KeyEvent::Released(_))
    }

    pub fn is_tap(&self) -> bool {
        matches!(*self, KeyEvent::Tap(_))
    }
}

include!(concat!(env!("OUT_DIR"), "/keys.rs"));
