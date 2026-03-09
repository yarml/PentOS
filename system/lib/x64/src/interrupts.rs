pub mod gate;
pub mod stackframe;

use {
    crate::{
        interrupts::gate::{InterruptHandlerFn, InterruptHandlerWithErrCodeFn},
        mem::addr::{Address, VirtAddr},
    },
    core::{arch::asm, mem},
    gate::{InterruptGate, InterruptGateEntry},
};

pub struct InterruptDescriptorTable {
    table: [InterruptGateEntry; 256],
}

#[repr(C, packed)]
struct IDTPointer {
    limit: u16,
    idt: VirtAddr,
}

impl InterruptDescriptorTable {
    pub const fn new() -> Self {
        Self {
            table: [InterruptGateEntry::null(); 256],
        }
    }
}
macro_rules! attach_system {
    ($vector:expr, $name:ident) => {
        pub fn $name(&mut self, gate: InterruptGate<InterruptHandlerFn>) {
            self.table[$vector] = gate.encode();
        }
    };
}
macro_rules! attach_system_errcode {
    ($vector:expr, $name:ident) => {
        pub fn $name(&mut self, gate: InterruptGate<InterruptHandlerWithErrCodeFn>) {
            self.table[$vector] = gate.encode();
        }
    };
}

impl InterruptDescriptorTable {
    attach_system!(0, attach_divide_error);
    attach_system!(1, attach_debug_exception);
    attach_system!(2, attach_nmi_interrupt);
    attach_system!(3, attach_breakpoint);
    attach_system!(4, attach_overflow);
    attach_system!(5, attach_bound_range_exceeded);
    attach_system!(6, attach_invalid_opcode);
    attach_system!(7, attach_device_not_available);
    attach_system_errcode!(8, attach_double_fault);
    attach_system!(9, attach_coproc_segment_overrun);
    attach_system_errcode!(10, attach_invalid_tss);
    attach_system_errcode!(11, attach_segment_not_present);
    attach_system_errcode!(12, attach_stack_segment);
    attach_system_errcode!(13, attach_general_protection);
    attach_system_errcode!(14, attach_page_fault);
    attach_system!(16, attach_fpu_error);
    attach_system_errcode!(17, attach_alignment_check);
    attach_system!(18, attach_machine_check);
    attach_system!(19, attach_simd_exception);
    attach_system!(20, attach_vt_exception);
    attach_system_errcode!(21, attach_control_protection_exception);

    pub fn attach(&mut self, vector: u8, gate: InterruptGate<InterruptHandlerFn>) {
        assert!(vector >= 32);
        self.table[vector as usize] = gate.encode();
    }
    pub fn clear(&mut self, vector: u8) {
        self.table[vector as usize] = InterruptGateEntry::null();
    }
    pub fn find(&self) -> Option<u8> {
        self.table
            .iter()
            .skip(32)
            .enumerate()
            .find(|(_, e)| e.is_free())
            .map(|(v, _)| v as u8)
    }
}

impl InterruptDescriptorTable {
    /// # Safety
    /// Self IDT must only use selectors which come from the currently loaded GDT
    pub unsafe fn load(&self, offset: VirtAddr) {
        let idtr = IDTPointer {
            idt: offset + self as *const _ as usize,
            limit: (mem::size_of::<InterruptDescriptorTable>() - 1) as u16,
        };
        let idtrp = &idtr as *const _;
        unsafe {
            // SAFETY: Guarenteed by caller
            asm! {
                "lidt [{idtrp}]",
                idtrp = in(reg) idtrp,
            }
        }
    }

    pub fn load_null() {
        let idtr = IDTPointer {
            idt: VirtAddr::null(),
            limit: 0,
        };
        let idtrp = &idtr as *const _;
        unsafe {
            // SAFETY: Guarenteed by caller
            asm! {
                "lidt [{idtrp}]",
                idtrp = in(reg) idtrp,
            }
        }
    }
}

impl Default for InterruptDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

pub fn disable() {
    unsafe { asm!("cli") }
}

pub fn enable() {
    unsafe { asm!("sti") }
}

pub fn enable_and_halt() {
    unsafe {
        asm! {
            "sti",
            "hlt"
        }
    }
}
