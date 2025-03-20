use crate::allocator::PreBootAllocator;
use crate::misc;
use uefi::CStr16;
use uefi::Identify;
use uefi::boot;
use uefi::boot::SearchType;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;

// TODO: Load kernel from PentFS partition
pub fn load_kernel(allocator: &PreBootAllocator) -> &'static [u8] {
    let simple_fs_handle =
        *boot::locate_handle_buffer(SearchType::ByProtocol(&SimpleFileSystem::GUID))
            .expect("Failed to locate SimpleFileSystem protocol")
            .first()
            .expect("No SimpleFileSystem protocol found");
    let mut simple_fs = boot::open_protocol_exclusive::<SimpleFileSystem>(simple_fs_handle)
        .expect("Failed to open SimpleFileSystem protocol");
    let mut volume = simple_fs.open_volume().expect("Failed to open volume");

    let filename = "pentos.kernel";
    let mut file_buf = [0u16; 256];
    let filename_wide =
        CStr16::from_str_with_buf(filename, &mut file_buf).expect("Filename too long");
    let kernel_file = volume
        .open(&filename_wide, FileMode::Read, FileAttribute::empty())
        .expect("Failed to open kernel file");
    let mut kernel_file = kernel_file
        .into_regular_file()
        .expect("Kernek file is not a regular file");

    let file_size =
        misc::get_file_size(&mut kernel_file, allocator).expect("Failed to get kernel file size");
    let buffer = allocator
        .alloc_slice(file_size, 0u8, boot::MemoryType::LOADER_DATA)
        .expect("Failed to allocate buffer for kernel file");
    kernel_file
        .read(buffer)
        .expect("Failed to read kernel file");
    buffer
}

/// Farewell, until another boot time...
pub fn cede_control() -> ! {
    loop {}
}
