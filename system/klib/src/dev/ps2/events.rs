use keys::Key;

pub enum KeyEvent {
    Press(Key),
    Release(Key),
}