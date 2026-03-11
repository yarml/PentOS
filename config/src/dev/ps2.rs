/// Number of elements held in the PS/2 keyboard event queue
/// before events start being discarded from the oldest
pub const KEY_EVENT_QUEUE_SIZE: usize = 32;

/// Maximum number of attempts to resend a keyboard command before giving up on bad responses
///
/// Only used in the init phase, otherwise does not matter
pub const KB_BAD_RESPONSE_MAX_RETRIES: usize = 4;
