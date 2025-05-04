use crate::prot::PrivilegeLevel;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SegmentDescriptor {
    SystemSegment, // TODO
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

impl SegmentDescriptor {
    pub const fn encode(&self) -> (SegmentDescriptorEntry, Option<SegmentDescriptorEntry>) {
        match self {
            SegmentDescriptor::SystemSegment => todo!(),
            SegmentDescriptor::AccessSegment { exec, dpl } => {
                (SegmentDescriptorEntry::flat(*exec, *dpl), None)
            }
            SegmentDescriptor::Null => (SegmentDescriptorEntry::null(), None),
        }
    }

    pub const fn dpl(&self) -> PrivilegeLevel {
        match self {
            SegmentDescriptor::SystemSegment => todo!(),
            SegmentDescriptor::AccessSegment { dpl, .. } => *dpl,
            SegmentDescriptor::Null => panic!("Attempt to get dpl of NULL descriptor"),
        }
    }
}

impl SegmentDescriptorEntry {
    #[inline]
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
    #[inline]
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
            base_high: 0,
            base_low: 0,
            base_middle: 0,
            flags_limit_high,
            limit_low: 0xFF,
        }
    }
}
