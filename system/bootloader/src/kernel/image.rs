use {
    crate::{allocator::PreBootAllocator, misc},
    elf::{Elf, ElfClass, ElfType},
    uefi::{
        CStr16, Identify,
        boot::{self, SearchType},
        proto::media::{
            file::{File, FileAttribute, FileMode},
            fs::SimpleFileSystem,
        },
    },
};

// TODO: Load kernel from PentFS partition
pub fn load_kernel(allocator: &PreBootAllocator) -> Elf<'static> {
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
        .open(filename_wide, FileMode::Read, FileAttribute::empty())
        .expect("Failed to open kernel file");
    let mut kernel_file = kernel_file
        .into_regular_file()
        .expect("Kernek file is not a regular file");

    let file_size =
        misc::get_file_size(&mut kernel_file, allocator).expect("Failed to get kernel file size");
    let buffer = unsafe {
        // SAFETY: Any u8 is valid
        allocator
            .alloc_slice(file_size, boot::MemoryType::LOADER_DATA)
            .expect("Failed to allocate buffer for kernel file")
            .assume_init_mut()
    };
    kernel_file
        .read(buffer)
        .expect("Failed to read kernel file");
    let elf = Elf::parse(buffer).expect("Failed to parse kernel");
    if elf.ty != ElfType::Executable {
        panic!("Kernel is not an executable");
    }
    if elf.ident.encoding != elf::DataEncoding::LittleEndian {
        panic!("Kernel is not little endian");
    }
    if elf.ident.class != ElfClass::Elf64 {
        panic!("Kernel is not 64-bit");
    }

    elf
}
