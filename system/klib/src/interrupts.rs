pub mod ioapic;
pub mod lapic;

mod handlers;

use {
    crate::interrupts::{
        handlers::{double_fault, generic_interrupt, nmi_interrupt},
        lapic::handlers::{error_interrupt, spurious_interrupt, timer_interrupt},
    },
    core::sync::atomic::{AtomicUsize, Ordering},
    spinlocks::mutex::SpinMutex,
    system::{
        hart::HartInfo,
        tss::{DF_IST, NMI_IST},
    },
    x64::{
        interrupts::{
            self, InterruptDescriptorTable,
            gate::{InterruptGate, InterruptHandlerFn},
        },
        mem::addr::{Address, VirtAddr},
    },
};

const VECTOR_TIMER: u8 = 0x20;
const VECTOR_LAPIC_SPURIOUS: u8 = 0x21;
const VECTOR_LAPIC_ERROR: u8 = 0x22;

const FREE_VECTOR_START: u8 = 0x23;

static IDT: SpinMutex<InterruptDescriptorTable> = SpinMutex::new(InterruptDescriptorTable::new());
static FREE_GATE: AtomicUsize = AtomicUsize::new(FREE_VECTOR_START as usize);

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

pub fn attach(handler: InterruptHandlerFn) -> usize {
    let hartinfo = HartInfo::get();
    let kernel_code_selector = hartinfo.kernel_code_selector();

    let mut idt = IDT.lock();

    let gate_n = FREE_GATE.fetch_add(1, Ordering::Relaxed);

    if gate_n >= 256 {
        panic!("allocated too many interrupt gates");
    }

    idt.attach(
        gate_n as u8,
        InterruptGate::simple(handler, kernel_code_selector),
    );

    gate_n
}
