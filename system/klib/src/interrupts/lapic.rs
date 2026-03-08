pub mod handlers;

use {
    crate::interrupts::{LAPIC_ERROR_VECTOR, LAPIC_SPURIOUS_VECTOR, TIMER_VECTOR},
    system::{hart::HartInfo, lapic_ptr},
    x64::lapic::TimerMode,
};

pub fn setup() {
    let lapic = lapic_ptr::standard();
    let hartinfo = HartInfo::get();

    lapic.set_timer_divisor(1);

    lapic.program_lvt_timer(TIMER_VECTOR, TimerMode::Periodic);
    lapic.program_spurious_vector(LAPIC_SPURIOUS_VECTOR);
    lapic.program_lvt_error(LAPIC_ERROR_VECTOR);

    lapic.set_timer_initial(hartinfo.lapic_10ms as u32);
}
