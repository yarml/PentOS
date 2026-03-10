use {log::debug, x64::interrupts::stackframe::InterruptStackFrame};

pub extern "x86-interrupt" fn ps2_kbd(_frame: InterruptStackFrame) {
    debug!("PS/2 Keyboard interrupt");
}
