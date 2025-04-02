use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use crate::allocator::PreBootAllocator;
use crate::misc;
use crate::virt_mmap;
use core::cmp::max;
use elf::Elf;
use elf::ElfClass;
use elf::ElfType;
use elf::SegmentType;
use uefi::CStr16;
use uefi::Identify;
use uefi::boot;
use uefi::boot::SearchType;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use x64::mem::addr::PhysAddr;
use x64::mem::frame::Frame;
use x64::mem::page::Page;
use x64::mem::paging::PagingRootEntry;
use x64::msr::pat::MemoryType;

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
    let buffer = allocator
        .alloc_slice(file_size, 0u8, boot::MemoryType::LOADER_DATA)
        .expect("Failed to allocate buffer for kernel file");
    kernel_file
        .read(buffer)
        .expect("Failed to read kernel file");
    let elf = Elf::parse(buffer).expect("Failed to parse kernel");
    if elf.ty != ElfType::SharedObject {
        panic!("Kernel is not an shared object");
    }
    if elf.ident.encoding != elf::DataEncoding::LittleEndian {
        panic!("Kernel is not little endian");
    }
    if elf.ident.class != ElfClass::Elf64 {
        panic!("Kernel is not 64-bit");
    }

    elf
}

pub fn map_kernel(
    kernel: &Elf<'static>,
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) {
    for segment in &kernel.program_header {
        if segment.ty == SegmentType::Load {
            let pg_count = segment.mem_size.next_multiple_of(4096) / 4096;
            let mut copied = 0;
            for i in 0..pg_count {
                let frame = Frame::containing(PhysAddr::new_truncate(
                    allocator.alloc([0; 4096]).expect("Out of memory") as *const _ as usize,
                ));
                if copied < segment.file_size {
                    let src = kernel.data.as_ptr() as u64 + segment.offset + copied as u64;
                    let dst = frame.boundary();
                    unsafe {
                        // SAFETY: We are copying from a valid memory region to a valid memory region
                        core::ptr::copy_nonoverlapping(
                            src as *const u8,
                            dst.as_mut_ptr(),
                            max(segment.file_size - copied, 4096),
                        );
                    }
                    copied += max(segment.file_size - copied, 4096);
                }
                let page = Page::containing(segment.vaddr + i * 4096);
                virt_mmap::map(
                    root_map,
                    allocator,
                    frame,
                    page,
                    segment.flags.write,
                    segment.flags.exec,
                    MemoryType::WriteBack,
                );
            }
        }
    }
}

/// Farewell, until another boot time...
pub fn cede_control() -> ! {
    todo!()
}
