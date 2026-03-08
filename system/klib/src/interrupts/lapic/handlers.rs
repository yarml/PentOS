use core::sync::atomic::{AtomicUsize, Ordering};

use {
    log::debug,
    system::{hart::HartInfo, lapic_ptr},
    x64::interrupts::stackframe::InterruptStackFrame,
};

pub extern "x86-interrupt" fn timer_interrupt(_frame: InterruptStackFrame) {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let lapic = lapic_ptr::standard();
    let hartinfo = HartInfo::get();

    if hartinfo.is_bsp() && COUNT.fetch_add(1, Ordering::Relaxed) == 100 {
        debug!("LAPIC TIMER INTERRUPT");
        COUNT.store(0, Ordering::Relaxed);
    }
    lapic.end_of_interrupt();
}

pub extern "x86-interrupt" fn spurious_interrupt(_frame: InterruptStackFrame) {
    todo!("LAPIC SPURIOUS INTERRUPT")
}

pub extern "x86-interrupt" fn error_interrupt(_frame: InterruptStackFrame) {
    todo!("LAPIC ERROR INTERRUPT")
}
