#[cfg(test)]
mod test;

pub mod pat;
pub mod pcid;
pub mod pk;

mod absent_entry;
mod map_entry;
mod raw_entry;
mod ref_entry;
mod root_entry;

pub use {
    map_entry::PagingMapEntry, raw_entry::PagingRawEntry, ref_entry::PagingReferenceEntry,
    root_entry::PagingRootEntry,
};
