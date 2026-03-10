use {
    crate::timers,
    core::{
        arch::x86_64::__cpuid,
        hint,
        sync::atomic::{AtomicBool, Ordering},
    },
    x64::lapic::LocalApic,
};

pub fn ticks_per_10ms() -> usize {
    calib_cpuid().unwrap_or_else(calib_timers)
}

fn calib_cpuid() -> Option<usize> {
    let max_leaf = __cpuid(0x0).eax;
    if max_leaf < 0x15 {
        return None;
    }

    let leaf = __cpuid(0x15);

    let numerator = leaf.ebx;
    let crystal_hz = leaf.ecx;

    if numerator == 0 || crystal_hz == 0 {
        return None;
    }

    let ticks_per_10ms = crystal_hz / 100;

    Some(ticks_per_10ms as usize)
}

fn calib_timers() -> usize {
    static SLEEP_USED: AtomicBool = AtomicBool::new(false);

    LocalApic::set_timer_divisor(1);

    while SLEEP_USED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        hint::spin_loop();
    }
    LocalApic::set_timer_initial(0xFFFF_FFFF);
    unsafe {
        // SAFETY: locking for one sleep at a time
        timers::sleep_us(10_000)
    };
    let remaining = LocalApic::get_timer();

    SLEEP_USED.store(false, Ordering::Relaxed);
    (0xFFFF_FFFF - remaining) as usize
}
