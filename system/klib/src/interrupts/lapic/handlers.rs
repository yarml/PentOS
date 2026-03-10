use {
    crate::{
        interrupts::lapic::TIMESTAMP,
        task::{self, sleep, suspend},
    },
    core::sync::atomic::Ordering,
    system::hart::HartInfo,
    x64::{interrupts::stackframe::InterruptStackFrame, lapic::LocalApic},
};

pub extern "x86-interrupt" fn timer_interrupt(_frame: InterruptStackFrame) {
    let hartinfo = HartInfo::get();

    if hartinfo.is_bsp() {
        TIMESTAMP.fetch_add(1, Ordering::Relaxed);
        task::spawn_urgent(sleep::wake);
        task::spawn_urgent(suspend::wake);
    }
    LocalApic::end_of_interrupt();
}

pub extern "x86-interrupt" fn spurious_interrupt(_frame: InterruptStackFrame) {
    todo!("LAPIC SPURIOUS INTERRUPT")
}

pub extern "x86-interrupt" fn error_interrupt(_frame: InterruptStackFrame) {
    todo!("LAPIC ERROR INTERRUPT")
}
