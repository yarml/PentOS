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
    attach_system_errcode!(8, double_fault);
    attach_system!(9, coproc_segment_overrun);
    attach_system_errcode!(10, invalid_tss);
    attach_system_errcode!(11, segment_not_present);
    attach_system_errcode!(12, stack_segment);
    attach_system_errcode!(13, general_protection);
    attach_system_errcode!(14, page_fault);
    attach_system!(16, fpu_error);
    attach_system_errcode!(17, alignment_check);
    attach_system!(18, machine_check);
    attach_system!(19, simd_exception);
    attach_system!(20, vt_exception);
    attach_system_errcode!(21, control_protection_exception);

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
    pub unsafe fn load(&self) {
        let idtr = IDTPointer {
            idt: VirtAddr::new_panic(self as *const _ as usize),
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
