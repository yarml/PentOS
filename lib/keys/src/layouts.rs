use crate::Key;

pub struct Layout {
    id: usize,
}

pub struct Character {
    normal: char,
    shifted: char,
    alternative: char,
    shifted_alternative: char,
}

impl Character {
    pub const fn new(
        normal: char,
        shifted: char,
        alternative: char,
        shifted_alternative: char,
    ) -> Self {
        Self {
            normal,
            shifted,
            alternative,
            shifted_alternative,
        }
    }

    pub const fn select(&self, shift: bool, alternative: bool) -> char {
        match (shift, alternative) {
            (false, false) => self.normal,
            (false, true) => self.alternative,
            (true, false) => self.shifted,
            (true, true) => self.shifted_alternative,
        }
    }
}

impl Layout {
    const fn of_id(id: usize) -> Self {
        Self { id }
    }

    pub fn resolve(&self, key: Key) -> Option<Character> {
        let universal = Self::resolve_universal(key);

        if universal.is_some() {
            return universal;
        }

        self.resolve_specific(key)
    }

    fn resolve_universal(key: Key) -> Option<Character> {
        match key {
            crate::KEY_SPACE => Some(Character::new(' ', ' ', ' ', ' ')),
            crate::KEY_KEYPAD_0 => Some(Character::new('0', '0', '0', '0')),
            crate::KEY_KEYPAD_1 => Some(Character::new('1', '1', '1', '1')),
            crate::KEY_KEYPAD_2 => Some(Character::new('2', '2', '2', '2')),
            crate::KEY_KEYPAD_3 => Some(Character::new('3', '3', '3', '3')),
            crate::KEY_KEYPAD_4 => Some(Character::new('4', '4', '4', '4')),
            crate::KEY_KEYPAD_5 => Some(Character::new('5', '5', '5', '5')),
            crate::KEY_KEYPAD_6 => Some(Character::new('6', '6', '6', '6')),
            crate::KEY_KEYPAD_7 => Some(Character::new('7', '7', '7', '7')),
            crate::KEY_KEYPAD_8 => Some(Character::new('8', '8', '8', '8')),
            crate::KEY_KEYPAD_9 => Some(Character::new('9', '9', '9', '9')),
            crate::KEY_KEYPAD_PLUS => Some(Character::new('+', '+', '+', '+')),
            crate::KEY_KEYPAD_MINUS => Some(Character::new('-', '-', '-', '-')),
            crate::KEY_KEYPAD_STAR => Some(Character::new('*', '*', '*', '*')),
            crate::KEY_KEYPAD_SLASH => Some(Character::new('/', '/', '/', '/')),
            _ => None,
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/layouts.rs"));
