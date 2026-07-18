use {
    klib::{
        interrupts::{self, ioapic},
        task,
    },
    x64::{interrupts::stackframe::InterruptStackFrame, lapic::LocalApic},
};

const DEFAULT_PS2_KEYBOARD_IRQ: usize = 1;

pub(crate) fn init_interrupt() {
    let ps2_interrupt_vector = interrupts::attach(ps2_kbd);
    ioapic::apply_irq_redirection(DEFAULT_PS2_KEYBOARD_IRQ, ps2_interrupt_vector);
}

extern "x86-interrupt" fn ps2_kbd(_frame: InterruptStackFrame) {
    task::spawn_urgent(crate::ps2_impl::on_scancode);
    LocalApic::end_of_interrupt();
}
