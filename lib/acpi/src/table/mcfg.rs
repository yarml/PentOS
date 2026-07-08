use {
    crate::table::{AcpiHeader, AcpiTable, MCFG_SIG},
    core::mem,
};

#[repr(C, packed)]
pub struct Mcfg {
    pub header: AcpiHeader,
    res0: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigSpacePhysicalMapEntry {
    pub base: u64,
    pub seg_group: u16,
    pub bus_start: u8,
    pub bus_end: u8,
    res0: u32,
}

pub struct McfgIter<'a> {
    mcfg: &'a Mcfg,
    index: usize,
}

impl Mcfg {
    const HEADER_SIZE: usize = mem::size_of::<AcpiHeader>();
    const ENTRY_SIZE: usize = mem::size_of::<ConfigSpacePhysicalMapEntry>();

    pub const fn entry_count(&self) -> usize {
        (self.header.len as usize - Self::HEADER_SIZE) / Self::ENTRY_SIZE
    }

    pub const fn entry_at(&self, index: usize) -> Option<ConfigSpacePhysicalMapEntry> {
        if index >= self.entry_count() {
            return None;
        }

        unsafe {
            let ptr = (self as *const _ as *const u8)
                .add(Self::HEADER_SIZE)
                .add(index * Self::ENTRY_SIZE)
                as *const ConfigSpacePhysicalMapEntry;
            Some(*ptr)
        }
    }

    pub const fn entries(&self) -> McfgIter<'_> {
        McfgIter {
            mcfg: self,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a Mcfg {
    type Item = ConfigSpacePhysicalMapEntry;
    type IntoIter = McfgIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries()
    }
}

impl<'a> Iterator for McfgIter<'a> {
    type Item = ConfigSpacePhysicalMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.mcfg.entry_at(self.index);
        self.index += 1;
        entry
    }
}

impl AcpiTable for Mcfg {
    const SIG: &'static [u8; 4] = MCFG_SIG;
}
