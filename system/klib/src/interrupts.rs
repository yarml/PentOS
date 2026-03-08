mod handlers;

use {
    crate::interrupts::handlers::{double_fault, nmi_interrupt},
    spinlocks::mutex::Mutex,
    system::{
        hart::HartInfo,
        tss::{DF_IST, NMI_IST},
    },
    x64::{
        interrupts::{InterruptDescriptorTable, gate::InterruptGate},
        mem::addr::{Address, VirtAddr},
    },
};

static IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());

pub(crate) fn setup() {
    let hartinfo = HartInfo::get();
    let kernel_code_selector = hartinfo.kernel_code_selector();

    let mut idt = IDT.lock();

    idt.attach_double_fault(InterruptGate::simple_ist(
        double_fault,
        kernel_code_selector,
        DF_IST,
    ));

    idt.attach_nmi_interrupt(InterruptGate::simple_ist(
        nmi_interrupt,
        kernel_code_selector,
        NMI_IST,
    ));
}

pub(crate) fn load() {
    let idt = IDT.lock();
    unsafe {
        // SAFETY: Kernel code segment from HartInfo comes from the onlny GDT we ever use
        // Null offset, since IDT is already in virtual space
        idt.load(VirtAddr::null())
    };
}
