pub mod handlers;

use {
    crate::interrupts::{LAPIC_ERROR_VECTOR, LAPIC_SPURIOUS_VECTOR, TIMER_VECTOR},
    core::sync::atomic::{AtomicUsize, Ordering},
    system::hart::HartInfo,
    x64::lapic::{LocalApic, TimerMode},
};

static TIMESTAMP: AtomicUsize = AtomicUsize::new(0);

pub fn setup() {
    let hartinfo = HartInfo::get();

    LocalApic::set_timer_divisor(1);

    LocalApic::program_lvt_timer(TIMER_VECTOR, TimerMode::Periodic);
    LocalApic::program_spurious_vector(LAPIC_SPURIOUS_VECTOR);
    LocalApic::program_lvt_error(LAPIC_ERROR_VECTOR);

    LocalApic::set_timer_initial(hartinfo.lapic_10ms as u32);
}

pub fn get_timestamp() -> usize {
    TIMESTAMP.load(Ordering::Relaxed)
}
