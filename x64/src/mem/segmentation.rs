pub mod descriptor;
pub mod selector;

use super::addr::Address;
use super::addr::VirtAddr;
use core::arch::asm;
use core::hint;
use core::mem;
use descriptor::SegmentDescriptor;
use descriptor::SegmentDescriptorEntry;
use selector::SegmentSelector;

#[repr(C)]
pub struct GlobalDescriptorTable<const N: usize> {
    table: [SegmentDescriptorEntry; N],
    len: usize,
}

#[repr(C, packed)]
pub struct GDTPointer {
    limit: u16,
    gdt: VirtAddr,
}

impl<const N: usize> GlobalDescriptorTable<N> {
    pub const fn empty() -> Self {
        Self {
            table: [SegmentDescriptor::Null.encode().0; N],
            len: 1, // We already have the mandatory NULL segment descriptor
        }
    }
}

impl<const N: usize> GlobalDescriptorTable<N> {
    pub fn push(&mut self, descriptor: SegmentDescriptor) -> SegmentSelector {
        let (lower_half, upper_half) = descriptor.encode();
        let addcount = if upper_half.is_some() { 2 } else { 1 };
        if self.len + addcount > N {
            hint::cold_path();
            panic!("Tried adding entry to already full GDT");
        }
        let index = self.len;
        self.table[self.len] = lower_half;
        self.len += 1;
        if let Some(upper_half) = upper_half {
            self.table[self.len] = upper_half;
            self.len += 1;
        }
        SegmentSelector::new(index as u16, descriptor.dpl())
    }
}

impl<const N: usize> GlobalDescriptorTable<N> {
    /// # Safety
    /// Caller must ensure the selectors come from this GDT, and that the priviliege levels
    /// are set appropriatly for the continued work of the system
    pub unsafe fn load(&self, code_selector: SegmentSelector, data_selector: SegmentSelector) {
        let gdtr = GDTPointer {
            gdt: VirtAddr::new_panic(self as *const _ as usize),
            limit: (self.len * mem::size_of::<SegmentDescriptorEntry>() - 1) as u16,
        };
        let gdtrp = &gdtr as *const _;
        unsafe {
            // # Safety
            // Guarenteed by caller
            asm! {
                "lgdt [{gdtrp}]",
                gdtrp = in(reg) gdtrp,
            }
            Self::load_selectors(code_selector, data_selector);
        }
    }

    /// # Safety
    /// Caller must ensure that selectors come from current loaded selectors
    pub unsafe fn load_selectors(code_selector: SegmentSelector, data_selector: SegmentSelector) {
        unsafe {
            // # Safety
            // Guarenteed by caller
            asm! {
                "mov ss, {ds:x}",
                "mov ds, {ds:x}",
                "mov es, {ds:x}",
                "mov fs, {ds:x}",
                "mov gs, {ds:x}",
                // The manipulation to load CS with a variable selector
                // Previously used in HeliumOS:
                // https://github.com/yarml/HeliumOS/blob/69d3c50916d261117131e5284b6ce50242e0c049/kernel/src/asm/gdt.asm#L6
                // The good old C days.
                "call 2", // -> manip
                "jmp 3", // -> leave
                "2:", // manip:
                "pop {scratch:r}",
                "push {cs:r}",
                "push {scratch:r}",
                "retfq",
                "3:", // leave:
                scratch = in(reg) 0,
                ds = in(reg) *data_selector,
                cs = in(reg) *code_selector,
            }
        }
    }
}
