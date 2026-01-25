use {
    crate::allocator::PreBootAllocator,
    uefi::{
        Error, Status,
        boot::MemoryType,
        proto::media::file::{File, FileInfo, RegularFile},
    },
};

pub fn get_file_size(file: &mut RegularFile, allocator: &PreBootAllocator) -> Option<usize> {
    let required_size = match file.get_info::<FileInfo>(&mut []).map_err(Error::split) {
        Err((Status::BUFFER_TOO_SMALL, Some(required_size))) => required_size,
        _ => return None,
    };
    let buffer = unsafe {
        // SAFETY: Any u8 is valid
        allocator
            .alloc_slice(required_size, MemoryType::LOADER_DATA)?
            .assume_init_mut()
    };
    let info = file.get_info::<FileInfo>(buffer).ok()?;
    let file_size = info.file_size() as usize;

    Some(file_size)
}
