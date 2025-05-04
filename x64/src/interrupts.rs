pub mod gate;
pub mod stackframe;

use crate::mem::addr::Address;
use crate::mem::addr::VirtAddr;
use core::arch::asm;
use core::mem;
use gate::InterruptGate;
use gate::InterruptGateEntry;
use gate::InterruptHandler;

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

impl InterruptDescriptorTable {
    pub fn attach<F: InterruptHandler>(&mut self, vector: usize, gate: InterruptGate<F>) {
        self.table[vector] = gate.encode();
    }
}

impl InterruptDescriptorTable {
    /// # Safety
    /// Self IDT must only use selectors which come from the currently loaded GDT
    pub unsafe fn load(&self) {
        let idtr = IDTPointer {
            idt: VirtAddr::new_panic(self as *const _ as usize),
            limit: (256 * mem::size_of::<InterruptGateEntry>() - 1) as u16,
        };
        let idtrp = &idtr as *const _;
        unsafe {
            // # Safety
            // Guarenteed by caller
            asm! {
                "lidt [{idtrp}]",
                idtrp = in(reg) idtrp,
            }
        }
    }
}
