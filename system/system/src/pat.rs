use x64::{
    mem::paging::pat::PatIndex,
    msr::pat::{MemoryType, Pat},
};

/// PatIndex assuming the correct standard table is setup from standard_pat() -> Pat
pub fn pat_index(mtype: MemoryType) -> PatIndex {
    match mtype {
        MemoryType::Uncacheable => PatIndex::new(2),
        MemoryType::WriteCombining => PatIndex::new(3),
        MemoryType::WriteThrough => PatIndex::new(1),
        MemoryType::WriteProtected => PatIndex::new(4),
        MemoryType::WriteBack => PatIndex::new(0),
        MemoryType::Uncached => PatIndex::new(5),
    }
}

pub fn standard_pat() -> Pat {
    let mut val = Pat::new();
    val.set(PatIndex::new(0), MemoryType::WriteBack)
        .set(PatIndex::new(1), MemoryType::WriteThrough)
        .set(PatIndex::new(2), MemoryType::Uncacheable)
        .set(PatIndex::new(3), MemoryType::WriteCombining)
        .set(PatIndex::new(4), MemoryType::WriteProtected)
        .set(PatIndex::new(5), MemoryType::Uncached)
        .set(PatIndex::new(6), MemoryType::Uncacheable)
        .set(PatIndex::new(7), MemoryType::WriteBack);
    val
}
