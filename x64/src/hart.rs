use core::arch::x86_64::__cpuid;

pub fn get_id() -> usize {
    (unsafe {
        // SAFETY: nothing to worry about
        __cpuid(1)
    }
    .ebx >> 24) as usize
}
