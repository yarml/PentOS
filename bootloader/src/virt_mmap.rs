mod map;
mod mappings;
mod paging;

pub use {
    map::{map, map_many},
    mappings::{apply_bootinfo_mapping, apply_id_and_off_mapping, apply_kbin_mapping},
    paging::{page_map_new, page_target_or_new, paging_root_new},
};
