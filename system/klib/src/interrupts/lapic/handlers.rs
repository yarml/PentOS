use {
    crate::{interrupts::lapic::TICK_LISTENERS, task},
    system::hart::HartInfo,
    x64::{interrupts::stackframe::InterruptStackFrame, lapic::LocalApic},
};

pub extern "x86-interrupt" fn timer_interrupt(_frame: InterruptStackFrame) {
    let hartinfo = HartInfo::get();
    if hartinfo.is_bsp() {
        TICK_LISTENERS
            .lock()
            .iter()
            .for_each(|l| task::spawn_urgent(*l));
    }
    LocalApic::end_of_interrupt();
}

pub extern "x86-interrupt" fn spurious_interrupt(_frame: InterruptStackFrame) {}

pub extern "x86-interrupt" fn error_interrupt(_frame: InterruptStackFrame) {
    todo!("LAPIC ERROR INTERRUPT")
}
