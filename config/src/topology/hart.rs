use x64::mem::MemorySize;

/// Maximum number of CPUs supported by the OS.
/// The OS will refuse to boot and instead panic if run on a machine
/// with a higher number of CPUs than specified here.
pub const MAX_HART_COUNT: usize = 16;

/// Maximum number of supported interrupt controllers by the OS.
/// The OS will refuse to boot and instead panic if run on a machine
/// with a higher number of interrupt controllers than specified here.
pub const MAX_INTCTL_COUNT: usize = 16;

/// Maximum number of retries to boot a secondary CPU.
/// Normally 2 and above is already fine.
pub const MAX_AP_RETRIES: usize = 4;

/// Kernel stack for each hart
/// Must be divisible by 2 Mib
pub const KSTACK_SIZE: MemorySize = MemorySize::new(2 * 1024 * 1024);

/// Stack size for special purposes
/// These include  double fault and NMI interrupts
/// Must be divisible by 4Kib
pub const SPECIAL_KSTACK_SIZE: MemorySize = MemorySize::new(4 * 1024);
