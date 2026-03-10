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

// Key definitions
// Bismillah
// Single byte - Numbers
pub const KEY_N1: Key = Key::simple(0x16);
pub const KEY_N2: Key = Key::simple(0x1E);
pub const KEY_N3: Key = Key::simple(0x26);
pub const KEY_N4: Key = Key::simple(0x25);
pub const KEY_N5: Key = Key::simple(0x2E);
pub const KEY_N6: Key = Key::simple(0x36);
pub const KEY_N7: Key = Key::simple(0x3D);
pub const KEY_N8: Key = Key::simple(0x3E);
pub const KEY_N9: Key = Key::simple(0x46);
pub const KEY_N0: Key = Key::simple(0x45);

// Single byte - Letters
pub const KEY_A: Key = Key::simple(0x1C);
pub const KEY_B: Key = Key::simple(0x32);
pub const KEY_C: Key = Key::simple(0x21);
pub const KEY_D: Key = Key::simple(0x23);
pub const KEY_E: Key = Key::simple(0x24);
pub const KEY_F: Key = Key::simple(0x2B);
pub const KEY_G: Key = Key::simple(0x34);
pub const KEY_H: Key = Key::simple(0x33);
pub const KEY_I: Key = Key::simple(0x43);
pub const KEY_J: Key = Key::simple(0x3B);
pub const KEY_K: Key = Key::simple(0x42);
pub const KEY_L: Key = Key::simple(0x4B);
pub const KEY_M: Key = Key::simple(0x3A);
pub const KEY_N: Key = Key::simple(0x31);
pub const KEY_O: Key = Key::simple(0x44);
pub const KEY_P: Key = Key::simple(0x4D);
pub const KEY_Q: Key = Key::simple(0x15);
pub const KEY_R: Key = Key::simple(0x2D);
pub const KEY_S: Key = Key::simple(0x1B);
pub const KEY_T: Key = Key::simple(0x2C);
pub const KEY_U: Key = Key::simple(0x3C);
pub const KEY_V: Key = Key::simple(0x2A);
pub const KEY_W: Key = Key::simple(0x1D);
pub const KEY_X: Key = Key::simple(0x22);
pub const KEY_Y: Key = Key::simple(0x35);
pub const KEY_Z: Key = Key::simple(0x1A);

// Single byte - F keys
pub const KEY_F1: Key = Key::simple(0x05);
pub const KEY_F2: Key = Key::simple(0x06);
pub const KEY_F3: Key = Key::simple(0x04);
pub const KEY_F4: Key = Key::simple(0x0C);
pub const KEY_F5: Key = Key::simple(0x03);
pub const KEY_F6: Key = Key::simple(0x0B);
pub const KEY_F7: Key = Key::simple(0x83);
pub const KEY_F8: Key = Key::simple(0x0A);
pub const KEY_F9: Key = Key::simple(0x01);
pub const KEY_F10: Key = Key::simple(0x09);
pub const KEY_F11: Key = Key::simple(0x78);
pub const KEY_F12: Key = Key::simple(0x07);

// Single byte - Modifiers
pub const KEY_SHIFT_LEFT: Key = Key::simple(0x12);
pub const KEY_SHIFT_RIGHT: Key = Key::simple(0x59);
pub const KEY_CONTROL_LEFT: Key = Key::simple(0x14);
pub const KEY_ALT_LEFT: Key = Key::simple(0x11);
pub const KEY_LOCK_CAPS: Key = Key::simple(0x58);
pub const KEY_LOCK_NUMBER: Key = Key::simple(0x77);
pub const KEY_LOCK_SCROLL: Key = Key::simple(0x7E);

// Single byte - Symbols
pub const KEY_BACKTICK: Key = Key::simple(0x0E);
pub const KEY_MINUS: Key = Key::simple(0x4E);
pub const KEY_EQUAL: Key = Key::simple(0x55);
pub const KEY_BRACKET_OPEN: Key = Key::simple(0x54);
pub const KEY_BRACKET_CLOSE: Key = Key::simple(0x5B);
pub const KEY_BACKSLASH: Key = Key::simple(0x5D);
pub const KEY_SEMICOLON: Key = Key::simple(0x4C);
pub const KEY_QUOTE: Key = Key::simple(0x52);
pub const KEY_COMMA: Key = Key::simple(0x41);
pub const KEY_DOT: Key = Key::simple(0x49);
pub const KEY_SLASH: Key = Key::simple(0x4A);

// Single byte - Whitespace & editing
pub const KEY_SPACE: Key = Key::simple(0x29);
pub const KEY_TAB: Key = Key::simple(0x0D);
pub const KEY_ENTER: Key = Key::simple(0x5A);
pub const KEY_BACKSPACE: Key = Key::simple(0x66);
pub const KEY_ESCAPE: Key = Key::simple(0x76);

// Single byte - Keypad
pub const KEY_KEYPAD_0: Key = Key::simple(0x70);
pub const KEY_KEYPAD_1: Key = Key::simple(0x69);
pub const KEY_KEYPAD_2: Key = Key::simple(0x72);
pub const KEY_KEYPAD_3: Key = Key::simple(0x7A);
pub const KEY_KEYPAD_4: Key = Key::simple(0x6B);
pub const KEY_KEYPAD_5: Key = Key::simple(0x73);
pub const KEY_KEYPAD_6: Key = Key::simple(0x74);
pub const KEY_KEYPAD_7: Key = Key::simple(0x6C);
pub const KEY_KEYPAD_8: Key = Key::simple(0x75);
pub const KEY_KEYPAD_9: Key = Key::simple(0x7D);
pub const KEY_KEYPAD_DOT: Key = Key::simple(0x71);
pub const KEY_KEYPAD_PLUS: Key = Key::simple(0x79);
pub const KEY_KEYPAD_MINUS: Key = Key::simple(0x7B);
pub const KEY_KEYPAD_STAR: Key = Key::simple(0x7C);

// Extended - Modifiers
pub const KEY_ALT_RIGHT: Key = Key::extended(0x11);
pub const KEY_CONTROL_RIGHT: Key = Key::extended(0x14);
pub const KEY_GUI_LEFT: Key = Key::extended(0x1F);
pub const KEY_GUI_RIGHT: Key = Key::extended(0x27);
pub const KEY_APPS: Key = Key::extended(0x2F);

// Extended - Navigation
pub const KEY_INSERT: Key = Key::extended(0x70);
pub const KEY_DELETE: Key = Key::extended(0x71);
pub const KEY_HOME: Key = Key::extended(0x6C);
pub const KEY_END: Key = Key::extended(0x69);
pub const KEY_PAGE_UP: Key = Key::extended(0x7D);
pub const KEY_PAGE_DOWN: Key = Key::extended(0x7A);
pub const KEY_CURSOR_UP: Key = Key::extended(0x75);
pub const KEY_CURSOR_DOWN: Key = Key::extended(0x72);
pub const KEY_CURSOR_LEFT: Key = Key::extended(0x6B);
pub const KEY_CURSOR_RIGHT: Key = Key::extended(0x74);

// Extended - Keypad
pub const KEY_KEYPAD_SLASH: Key = Key::extended(0x4A);
pub const KEY_KEYPAD_ENTER: Key = Key::extended(0x5A);

// Extended - ACPI
pub const KEY_ACPI_POWER: Key = Key::extended(0x37);
pub const KEY_ACPI_SLEEP: Key = Key::extended(0x3F);
pub const KEY_ACPI_WAKE: Key = Key::extended(0x5E);

// Extended - Multimedia
pub const KEY_MULTIMEDIA_TRACK_PREVIOUS: Key = Key::extended(0x15);
pub const KEY_MULTIMEDIA_TRACK_NEXT: Key = Key::extended(0x4D);
pub const KEY_MULTIMEDIA_PLAY_PAUSE: Key = Key::extended(0x34);
pub const KEY_MULTIMEDIA_STOP: Key = Key::extended(0x3B);
pub const KEY_MULTIMEDIA_MUTE: Key = Key::extended(0x23);
pub const KEY_MULTIMEDIA_VOLUME_UP: Key = Key::extended(0x32);
pub const KEY_MULTIMEDIA_VOLUME_DOWN: Key = Key::extended(0x21);
pub const KEY_MULTIMEDIA_CALCULATOR: Key = Key::extended(0x2B);
pub const KEY_MULTIMEDIA_MY_COMPUTER: Key = Key::extended(0x40);
pub const KEY_MULTIMEDIA_EMAIL: Key = Key::extended(0x48);
pub const KEY_MULTIMEDIA_MEDIA_SELECT: Key = Key::extended(0x50);

// Extended - WWW
pub const KEY_MULTIMEDIA_WWW_SEARCH: Key = Key::extended(0x10);
pub const KEY_MULTIMEDIA_WWW_FAVOURITES: Key = Key::extended(0x18);
pub const KEY_MULTIMEDIA_WWW_REFRESH: Key = Key::extended(0x20);
pub const KEY_MULTIMEDIA_WWW_STOP: Key = Key::extended(0x28);
pub const KEY_MULTIMEDIA_WWW_FORWARD: Key = Key::extended(0x30);
pub const KEY_MULTIMEDIA_WWW_BACK: Key = Key::extended(0x38);
pub const KEY_MULTIMEDIA_WWW_HOME: Key = Key::extended(0x3A);

// Special multi-byte
pub const KEY_PRINT_SCREEN: Key = Key::print_screen();
pub const KEY_PAUSE: Key = Key::pause();

// Alhamdulillah

// And then, we finish it with a universal list
pub const KEY_LIST: &[Key] = &[
    // Numbers
    KEY_N1,
    KEY_N2,
    KEY_N3,
    KEY_N4,
    KEY_N5,
    KEY_N6,
    KEY_N7,
    KEY_N8,
    KEY_N9,
    KEY_N0,
    // Letters
    KEY_A,
    KEY_B,
    KEY_C,
    KEY_D,
    KEY_E,
    KEY_F,
    KEY_G,
    KEY_H,
    KEY_I,
    KEY_J,
    KEY_K,
    KEY_L,
    KEY_M,
    KEY_N,
    KEY_O,
    KEY_P,
    KEY_Q,
    KEY_R,
    KEY_S,
    KEY_T,
    KEY_U,
    KEY_V,
    KEY_W,
    KEY_X,
    KEY_Y,
    KEY_Z,
    // F keys
    KEY_F1,
    KEY_F2,
    KEY_F3,
    KEY_F4,
    KEY_F5,
    KEY_F6,
    KEY_F7,
    KEY_F8,
    KEY_F9,
    KEY_F10,
    KEY_F11,
    KEY_F12,
    // Modifiers
    KEY_SHIFT_LEFT,
    KEY_SHIFT_RIGHT,
    KEY_CONTROL_LEFT,
    KEY_CONTROL_RIGHT,
    KEY_ALT_LEFT,
    KEY_ALT_RIGHT,
    KEY_GUI_LEFT,
    KEY_GUI_RIGHT,
    KEY_LOCK_CAPS,
    KEY_LOCK_NUMBER,
    KEY_LOCK_SCROLL,
    KEY_APPS,
    // Symbols
    KEY_BACKTICK,
    KEY_MINUS,
    KEY_EQUAL,
    KEY_BRACKET_OPEN,
    KEY_BRACKET_CLOSE,
    KEY_BACKSLASH,
    KEY_SEMICOLON,
    KEY_QUOTE,
    KEY_COMMA,
    KEY_DOT,
    KEY_SLASH,
    // Whitespace & editing
    KEY_SPACE,
    KEY_TAB,
    KEY_ENTER,
    KEY_BACKSPACE,
    KEY_ESCAPE,
    // Navigation
    KEY_INSERT,
    KEY_DELETE,
    KEY_HOME,
    KEY_END,
    KEY_PAGE_UP,
    KEY_PAGE_DOWN,
    KEY_CURSOR_UP,
    KEY_CURSOR_DOWN,
    KEY_CURSOR_LEFT,
    KEY_CURSOR_RIGHT,
    // Keypad
    KEY_KEYPAD_0,
    KEY_KEYPAD_1,
    KEY_KEYPAD_2,
    KEY_KEYPAD_3,
    KEY_KEYPAD_4,
    KEY_KEYPAD_5,
    KEY_KEYPAD_6,
    KEY_KEYPAD_7,
    KEY_KEYPAD_8,
    KEY_KEYPAD_9,
    KEY_KEYPAD_DOT,
    KEY_KEYPAD_PLUS,
    KEY_KEYPAD_MINUS,
    KEY_KEYPAD_STAR,
    KEY_KEYPAD_SLASH,
    KEY_KEYPAD_ENTER,
    // ACPI
    KEY_ACPI_POWER,
    KEY_ACPI_SLEEP,
    KEY_ACPI_WAKE,
    // Multimedia
    KEY_MULTIMEDIA_TRACK_PREVIOUS,
    KEY_MULTIMEDIA_TRACK_NEXT,
    KEY_MULTIMEDIA_PLAY_PAUSE,
    KEY_MULTIMEDIA_STOP,
    KEY_MULTIMEDIA_MUTE,
    KEY_MULTIMEDIA_VOLUME_UP,
    KEY_MULTIMEDIA_VOLUME_DOWN,
    KEY_MULTIMEDIA_CALCULATOR,
    KEY_MULTIMEDIA_MY_COMPUTER,
    KEY_MULTIMEDIA_EMAIL,
    KEY_MULTIMEDIA_MEDIA_SELECT,
    // WWW
    KEY_MULTIMEDIA_WWW_SEARCH,
    KEY_MULTIMEDIA_WWW_FAVOURITES,
    KEY_MULTIMEDIA_WWW_REFRESH,
    KEY_MULTIMEDIA_WWW_STOP,
    KEY_MULTIMEDIA_WWW_FORWARD,
    KEY_MULTIMEDIA_WWW_BACK,
    KEY_MULTIMEDIA_WWW_HOME,
    // Special
    KEY_PRINT_SCREEN,
    KEY_PAUSE,
];
