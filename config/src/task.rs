/// Maximum number of urgent tasks that can be scheduled at once
///
/// Urgent tasks are the mechanism interrupts use to delegate work and keep
/// their procedure as small as possible.
pub const MAX_URGENT_TASK_COUNT: usize = 1024;