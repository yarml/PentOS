use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use crate::allocator::PreBootAllocator;
use crate::infoarea::allocate_info_space;
use crate::misc;
use crate::virt_mmap;
use boot_protocol::STACK_SIZE;
use boot_protocol::kernel_meta::KernelMeta;
use x64::mem::page::size::PageSize;
use core::arch::asm;
use core::cmp::max;
use core::hint;
use core::mem;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use elf::Elf;
use elf::ElfClass;
use elf::ElfType;
use elf::SegmentType;
use mman::phys::PhysicalAllocationRequest;
use mman::phys::PhysicalMemoryAllocator;
use spinlocks::once::Once;
use uefi::CStr16;
use uefi::Identify;
use uefi::boot;
use uefi::boot::SearchType;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use x64::lapic;
use x64::mem::addr::Address;
use x64::mem::addr::PhysAddr;
use x64::mem::addr::VirtAddr;
use x64::mem::frame::Frame;
use x64::mem::page::Page;
use x64::mem::page::size::Page4KiB;
use x64::mem::paging::PagingRootEntry;
use x64::msr::pat::MemoryType;

struct ApInfo {
    pub ap_entry: VirtAddr,
    pub stack_base: VirtAddr,
}

static AP_CEDE: Once<ApInfo> = Once::new();
static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);

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
                let frame = Frame::containing(PhysAddr::new_panic(
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

pub fn alloc_stack(
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> VirtAddr {
    let stack = Page::containing(allocate_info_space(STACK_SIZE));
    let pg_count = STACK_SIZE.next_multiple_of(4096) / 4096;
    for i in 0..pg_count {
        let frame = Frame::containing(
            allocator
                .maybe_allocate(PhysicalAllocationRequest::size_align(
                    Page4KiB::SIZE,
                    Page4KiB::SIZE,
                ))
                .expect("Out of memory").start(),
        );
        let page = stack + i;
        virt_mmap::map(
            root_map,
            allocator,
            frame,
            page,
            true,
            false,
            MemoryType::WriteBack,
        );
    }
    stack.boundary() + STACK_SIZE
}

pub fn bsp_cede_control(kernel: &Elf<'static>, stack: VirtAddr) -> ! {
    let entry = kernel.entry;
    let entry = entry.as_usize();
    let entry: extern "C" fn() -> KernelMeta = unsafe {
        // SAFETY: sometimes rust sucks
        mem::transmute(entry)
    };
    let meta = entry();

    let bsp_entry = meta.bsp_entry.as_usize();

    AP_CEDE.init(|| ApInfo {
        ap_entry: meta.ap_entry,
        stack_base: stack,
    });
    while AP_REMAINING.load(Ordering::Relaxed) > 0 {
        hint::spin_loop();
    }

    let stack = stack.as_usize();

    do_jump(stack, bsp_entry);
}

pub fn ap_cede_control() {
    AP_REMAINING.fetch_add(1, Ordering::Relaxed);
    while AP_CEDE.get().is_none() {
        hint::spin_loop();
    }

    let ap_info = AP_CEDE.get().unwrap();

    let ap_entry = ap_info.ap_entry.as_usize();
    let stack = ap_info.stack_base.as_usize() + STACK_SIZE * lapic::id_cpuid();

    AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
    do_jump(stack, ap_entry);
}

#[allow(unreachable_code, unused_variables)]
fn do_jump(stack: usize, dest: usize) -> ! {
    loop {
        hint::spin_loop();
    }
    unsafe {
        asm!(
            "mov rsp, {0}",
            "jmp {1}",
            in(reg) stack,
            in(reg) dest,
            options(noreturn)
        );
    }
}
