pub mod handlers;

use {
    crate::interrupts::{VECTOR_LAPIC_ERROR, VECTOR_LAPIC_SPURIOUS, VECTOR_TIMER},
    system::hart::HartInfo,
    x64::lapic::{LocalApic, TimerMode},
};

pub fn setup() {
    let hartinfo = HartInfo::get();

    LocalApic::set_timer_divisor(1);

    LocalApic::program_lvt_timer(VECTOR_TIMER, TimerMode::Periodic);
    LocalApic::program_spurious_vector(VECTOR_LAPIC_SPURIOUS);
    LocalApic::program_lvt_error(VECTOR_LAPIC_ERROR);

    LocalApic::set_timer_initial(hartinfo.lapic_10ms as u32);
}
