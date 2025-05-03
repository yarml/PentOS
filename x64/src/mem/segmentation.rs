pub mod descriptor;
pub mod selector;

use core::arch::asm;
use core::hint;
use core::mem;
use descriptor::SegmentDescriptor;
use descriptor::SegmentDescriptorEntry;
use selector::SegmentSelector;

use super::addr::VirtAddr;

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
    pub unsafe fn load(&self, code_segment: SegmentSelector, data_segment: SegmentSelector) {
        let gdtr = GDTPointer {
            gdt: VirtAddr::new_truncate(self as *const _ as usize),
            limit: (self.len * mem::size_of::<SegmentDescriptorEntry>() - 1) as u16,
        };
        let gdtrp = &gdtr as *const _;
        unsafe {
            // # Safety
            // Guarenteed by caller
            asm! {
                "lgdt [{gdtrp}]",
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
                "pop {gdtrp}",
                "push {cs:r}",
                "push {gdtrp}",
                "3:", // leave:

                gdtrp = in(reg) gdtrp,
                ds = in(reg) *data_segment,
                cs = in(reg) *code_segment,
            }
        }
    }
}
