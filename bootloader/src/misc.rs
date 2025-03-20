use uefi::Error;
use uefi::Status;
use uefi::boot::MemoryType;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileInfo;
use uefi::proto::media::file::RegularFile;
use crate::allocator::PreBootAllocator;

pub fn get_file_size(file: &mut RegularFile, allocator: &PreBootAllocator) -> Option<usize> {
    let required_size = match file.get_info::<FileInfo>(&mut []).map_err(Error::split) {
        Err((Status::BUFFER_TOO_SMALL, Some(required_size))) => required_size,
        _ => return None,
    };
    let buffer = allocator.alloc_slice(required_size, 0u8, MemoryType::LOADER_DATA)?;
    let info = file.get_info::<FileInfo>(buffer).ok()?;
    let file_size = info.file_size() as usize;

    Some(file_size)
}
