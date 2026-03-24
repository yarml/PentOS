use {
    crate::format::{GptHeader, PartitionEntry},
    alloc::boxed::Box,
};

pub struct HeaderCache {
    pub lba: u64,
    pub header: Box<[u8]>,
    pub partlist: Box<[u8]>,
}

impl HeaderCache {
    pub fn header(&self) -> &GptHeader {
        GptHeader::from_raw(&self.header)
    }
    pub fn header_mut(&mut self) -> &mut GptHeader {
        GptHeader::from_raw_mut(&mut self.header)
    }

    pub fn partlist(&self) -> &[PartitionEntry] {
        let cap = self.header().partlist_cap();
        PartitionEntry::from_raw(&self.partlist, cap)
    }
    pub fn partlist_mut(&mut self) -> &mut [PartitionEntry] {
        let cap = self.header().partlist_cap();
        PartitionEntry::from_raw_mut(&mut self.partlist, cap)
    }
}
