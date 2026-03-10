use crate::{
    mem::{
        addr::{Address, VirtAddr},
        segmentation::task_state::TaskStateSegment,
    },
    prot::PrivilegeLevel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentDescriptor {
    TaskStateSegment { base: VirtAddr },
    AccessSegment { exec: bool, dpl: PrivilegeLevel },
    Null,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SegmentDescriptorEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    flags_limit_high: u8,
    base_high: u8,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ExtendedSegmentDescriptorEntry {
    base: u32,
    res0: u32,
}

impl SegmentDescriptor {
    pub const fn encode(
        &self,
    ) -> (
        SegmentDescriptorEntry,
        Option<ExtendedSegmentDescriptorEntry>,
    ) {
        match self {
            SegmentDescriptor::TaskStateSegment { base } => (
                SegmentDescriptorEntry::tss(*base),
                Some(ExtendedSegmentDescriptorEntry::tss(*base)),
            ),
            SegmentDescriptor::AccessSegment { exec, dpl } => {
                (SegmentDescriptorEntry::flat(*exec, *dpl), None)
            }
            SegmentDescriptor::Null => (SegmentDescriptorEntry::null(), None),
        }
    }

    pub const fn dpl(&self) -> PrivilegeLevel {
        match self {
            SegmentDescriptor::TaskStateSegment { .. } => PrivilegeLevel::Kernel,
            SegmentDescriptor::AccessSegment { dpl, .. } => *dpl,
            SegmentDescriptor::Null => PrivilegeLevel::Kernel,
        }
    }
}

impl SegmentDescriptorEntry {
    #[inline(always)]
    const fn null() -> Self {
        SegmentDescriptorEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            flags_limit_high: 0,
            base_high: 0,
        }
    }
    #[inline(always)]
    const fn flat(exec: bool, dpl: PrivilegeLevel) -> Self {
        let execbit = if exec { 1 } else { 0 };
        let sizebit = if exec { 0 } else { 1 };
        let access = 1 // accessed
                    | 1 << 1 // code:readable, data:writable
                    | execbit << 3
                    | 1 << 4 // type:access
                    | (dpl as u8) << 5
                    | 1 << 7 // present
                    ;
        let flags = execbit << 1
                    | sizebit << 2
                    | 1 << 3 // Granularity=4KiB
                    ; // code:nonconforming, data:growdown
        let flags_limit_high = flags << 4 | 0xF;
        Self {
            access,
            base_low: 0,
            base_middle: 0,
            base_high: 0,
            flags_limit_high,
            limit_low: 0xFFFF,
        }
    }

    #[inline(always)]
    const fn tss(base: VirtAddr) -> Self {
        let limit = core::mem::size_of::<TaskStateSegment>() - 1;
        let limit_low = (limit & 0xFFFF) as u16;
        let flags_limit_high = ((limit >> 16) & 0xF) as u8; // Flags = 0 (No G)

        let base = base.as_u64();
        let base_low = (base & 0xFFFF) as u16;
        let base_middle = ((base >> 16) & 0xFF) as u8;
        let base_high = ((base >> 24) & 0xFF) as u8;

        Self {
            access: 0x89, // System, Present, DPL=0, Type = 9 (TSS)
            limit_low,
            flags_limit_high,
            base_low,
            base_middle,
            base_high,
        }
    }

    pub const fn as_u64(&self) -> u64 {
        *unsafe {
            // SAFETY: SegmentDescriptorEntry is 8 bytes repr(C)
            &*(self as *const Self as *const u64)
        }
    }
}

impl ExtendedSegmentDescriptorEntry {
    #[inline(always)]
    const fn tss(base: VirtAddr) -> Self {
        let base_high2 = (base.as_u64() >> 32) as u32;
        Self {
            base: base_high2,
            res0: 0,
        }
    }

    pub const fn as_u64(&self) -> u64 {
        *unsafe {
            // SAFETY: SegmentDescriptorEntry is 8 bytes repr(C)
            &*(self as *const Self as *const u64)
        }
    }
}
