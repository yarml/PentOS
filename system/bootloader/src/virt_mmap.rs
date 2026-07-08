mod map;
mod mappings;
mod paging;

pub use {
    map::{map, map_many, map_optimal},
    mappings::{
        apply_bootinfo_mapping, apply_id_and_off_mapping, apply_ioapic_mappings,
        apply_kbin_mapping, apply_legacy_mem_mapping, apply_pcie_mappings,
    },
    paging::{page_map_new, page_target_or_new, paging_root_new},
};
