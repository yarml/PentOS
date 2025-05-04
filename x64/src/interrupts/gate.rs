use super::stackframe::InterruptStackFrame;
use crate::mem::addr::Address;
use crate::mem::addr::VirtAddr;
use crate::mem::segmentation::selector::SegmentSelector;
use crate::prot::PrivilegeLevel;
use core::num::NonZeroU8;

// A bit (too much) plagiarism from x86_64 crate
pub type InterruptHandlerFn = extern "x86-interrupt" fn(InterruptStackFrame);
pub type InterruptHandlerWithErrCode = extern "x86-interrupt" fn(InterruptStackFrame, u64);

/// # Safety
/// addr must return a valid VirtAddr to an interrupt handler
pub unsafe trait InterruptHandler: Clone + Copy {
    fn addr(self) -> VirtAddr;
}

macro_rules! impl_interrupt_handler {
    ($f:ty) => {
        unsafe impl InterruptHandler for $f {
            #[inline]
            fn addr(self) -> VirtAddr {
                VirtAddr::new_panic(self as usize)
            }
        }
    };
}

impl_interrupt_handler!(InterruptHandlerFn);
impl_interrupt_handler!(InterruptHandlerWithErrCode);

#[derive(Clone, Copy)]
pub struct InterruptGate<F> {
    handler: F,
    selector: SegmentSelector,
    ist: Option<NonZeroU8>,
    ty: GateType,
    dpl: PrivilegeLevel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GateType {
    Interrupt = 0xE,
    Trap = 0xF,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct InterruptGateEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    access: u8,
    offset_middle: u16,
    offset_high: u32,
    res0: u32,
}

impl InterruptGateEntry {
    pub const fn null() -> Self {
        Self {
            access: 0,
            ist: 0,
            offset_high: 0,
            offset_low: 0,
            offset_middle: 0,
            res0: 0,
            selector: 0,
        }
    }
    const fn mkentry(
        handler: VirtAddr,
        segment_selector: SegmentSelector,
        ist: Option<NonZeroU8>,
        ty: GateType,
        dpl: PrivilegeLevel,
    ) -> Self {
        let offset_low = (handler.as_usize() & 0xFF) as u16;
        let offset_middle = ((handler.as_usize() >> 16) & 0xFF) as u16;
        let offset_high = (handler.as_usize() >> 32 & 0xFFFFFFFF) as u32;

        let ist = if let Some(ist) = ist { ist.get() } else { 0 };
        let access = ty as u8
                    | (dpl as u8) << 5
                    | 1 << 7 // present
                    ;

        Self {
            offset_low,
            offset_middle,
            offset_high,
            selector: segment_selector.get(),
            ist,
            access,
            res0: 0,
        }
    }
}

impl<F: InterruptHandler> InterruptGate<F> {
    pub fn encode(&self) -> InterruptGateEntry {
        InterruptGateEntry::mkentry(
            self.handler.addr(),
            self.selector,
            self.ist,
            self.ty,
            self.dpl,
        )
    }
}
