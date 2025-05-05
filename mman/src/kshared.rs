use x64::mem::page::size::Page512GiB;
use x64::mem::paging::PagingReferenceEntry;
use x64::mem::paging::PagingRootEntry;

struct SharedKernelMapping {
    pg_entries: [PagingReferenceEntry<Page512GiB>; 128],
}

impl SharedKernelMapping {
    fn apply(&self, root: PagingRootEntry) {
        let target = &mut root.target_mut()[256..256 + 128];
        target.copy_from_slice(&self.pg_entries);
    }
}
