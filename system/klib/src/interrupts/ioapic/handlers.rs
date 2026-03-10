use {
    crate::{dev, task},
    x64::{interrupts::stackframe::InterruptStackFrame, lapic::LocalApic},
};

pub extern "x86-interrupt" fn ps2_kbd(_frame: InterruptStackFrame) {
    task::spawn_urgent(dev::ps2::on_key_event);
    LocalApic::end_of_interrupt();
}
