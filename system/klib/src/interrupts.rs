mod handlers;
mod ioapic;
mod lapic;

use {
    crate::interrupts::{
        handlers::{double_fault, generic_interrupt, nmi_interrupt},
        ioapic::handlers::ps2_kbd,
        lapic::handlers::{error_interrupt, spurious_interrupt, timer_interrupt},
    },
    spinlocks::mutex::Mutex,
    system::{
        hart::HartInfo,
        tss::{DF_IST, NMI_IST},
    },
    x64::{
        interrupts::{self, InterruptDescriptorTable, gate::InterruptGate},
        mem::addr::{Address, VirtAddr},
    },
};

const VECTOR_TIMER: u8 = 0x20;
const VECTOR_LAPIC_SPURIOUS: u8 = 0x21;
const VECTOR_LAPIC_ERROR: u8 = 0x22;
const VECTOR_PS2_KEYBOARD: u8 = 0x23;

/// We attach a general purpose interrupt handler from here up to & including 255
const FREE_VECTOR_START: u8 = 0x24;

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

    idt.attach(
        VECTOR_TIMER,
        InterruptGate::simple(timer_interrupt, kernel_code_selector),
    );
    idt.attach(
        VECTOR_LAPIC_SPURIOUS,
        InterruptGate::simple(spurious_interrupt, kernel_code_selector),
    );
    idt.attach(
        VECTOR_LAPIC_ERROR,
        InterruptGate::simple(error_interrupt, kernel_code_selector),
    );
    idt.attach(
        VECTOR_PS2_KEYBOARD,
        InterruptGate::simple(ps2_kbd, kernel_code_selector),
    );

    for i in FREE_VECTOR_START..=255 {
        idt.attach(
            i,
            InterruptGate::simple(generic_interrupt, kernel_code_selector),
        );
    }

    ioapic::init();
}

pub(crate) fn load() {
    let idt = IDT.lock();
    unsafe {
        // SAFETY: Kernel code segment from HartInfo comes from the onlny GDT we ever use
        // Null offset, since IDT is already in virtual space
        idt.load(VirtAddr::null())
    };
}

pub(crate) fn enable() {
    interrupts::enable();
    lapic::setup();
}
