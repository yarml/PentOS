pub mod handlers;

use {
    crate::{
        interrupts::{VECTOR_LAPIC_ERROR, VECTOR_LAPIC_SPURIOUS, VECTOR_TIMER},
        task::urgent_task::UrgentTask,
    },
    alloc::vec::Vec,
    spinlocks::mutex::SpinMutex,
    system::hart::HartInfo,
    x64::lapic::{LocalApic, TimerMode},
};

static TICK_LISTENERS: SpinMutex<Vec<UrgentTask>> = SpinMutex::new(Vec::new());

pub fn setup() {
    let hartinfo = HartInfo::get();

    LocalApic::set_timer_divisor(1);

    LocalApic::program_lvt_timer(VECTOR_TIMER, TimerMode::Periodic);
    LocalApic::program_spurious_vector(VECTOR_LAPIC_SPURIOUS);
    LocalApic::program_lvt_error(VECTOR_LAPIC_ERROR);

    LocalApic::set_timer_initial(hartinfo.lapic_10ms as u32);
}

pub fn attach_tick_listener(listener: UrgentTask) {
    TICK_LISTENERS.lock().push(listener);
}
