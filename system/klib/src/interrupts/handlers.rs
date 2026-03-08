use x64::interrupts::stackframe::InterruptStackFrame;

pub extern "x86-interrupt" fn double_fault(_frame: InterruptStackFrame, _code: u64) {
    // TODO: Notify other harts that we're ducked
    panic!("DOUBLE FAULT");
}

pub extern "x86-interrupt" fn nmi_interrupt(_frame: InterruptStackFrame) {
    todo!("NMI Interrupt");
}
