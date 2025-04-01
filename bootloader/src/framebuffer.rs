use crate::allocator::ALLOCATOR_CAP;
use crate::allocator::PostBootAllocator;
use crate::infoarea::allocate_info_space;
use crate::virt_mmap::map;
use boot_protocol::framebuffer::FramebufferInfo;
use core::mem;
use core::slice;
use uefi::Identify;
use uefi::boot;
use uefi::boot::SearchType;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::gop::Mode;
use uefi::proto::console::gop::PixelFormat;
use x64::framebuffer::PixelMode;
use x64::mem::MemorySize;
use x64::mem::addr::PhysAddr;
use x64::mem::frame::Frame;
use x64::mem::page::Page;
use x64::mem::paging::PagingRootEntry;

pub struct PrimaryFramebufferInfo {
    base: PhysAddr,
    size: MemorySize,
    width: usize,
    height: usize,
    stride: usize,
    mode: PixelMode,
}

pub fn init() -> PrimaryFramebufferInfo {
    let handle = *boot::locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID))
        .expect("Failed to locate GOP")
        .first()
        .expect("No GOP found");
    let mut gop =
        boot::open_protocol_exclusive::<GraphicsOutput>(handle).expect("Couldn't open GOP");

    let best_mode = gop
        .modes()
        .filter(|mode| {
            mode.info().pixel_format() == PixelFormat::Bgr
                || mode.info().pixel_format() == PixelFormat::Rgb
        })
        .fold(None, |best_mode: Option<Mode>, this_mode| {
            if best_mode.is_none_or(|best_mode| {
                let (best_width, best_height) = best_mode.info().resolution();
                let (this_width, this_height) = this_mode.info().resolution();
                let best_area = best_width * best_height;
                let this_area = this_width * this_height;
                best_area < this_area
            }) {
                Some(this_mode)
            } else {
                Some(best_mode.unwrap())
            }
        });

    let Some(best_mode) = best_mode else {
        panic!("No suitable GOP mode found");
    };
    gop.set_mode(&best_mode).expect("Couldn't set GOP mode");
    let info = best_mode.info();
    let (width, height) = info.resolution();
    let stride = info.stride();

    let mut fb = gop.frame_buffer();
    let base = PhysAddr::new_truncate(fb.as_mut_ptr() as usize);
    let size = MemorySize::new(fb.size());

    PrimaryFramebufferInfo {
        base,
        size,
        width,
        height,
        stride,
        mode: match info.pixel_format() {
            PixelFormat::Bgr => PixelMode::BgrRs,
            PixelFormat::Rgb => PixelMode::RgbRs,
            _ => unimplemented!("Unsupported pixel format"),
        },
    }
}

pub fn postboot_init(
    primary: PrimaryFramebufferInfo,
    root_map: PagingRootEntry,
    allocator: &mut PostBootAllocator<ALLOCATOR_CAP>,
) -> FramebufferInfo {
    let buffer = allocator
        .alloc_raw(*primary.size, 0x1000)
        .expect("Out of memory");
    let buffer_frame_start = Frame::containing(buffer);
    let buffer = unsafe {
        // SAFETY: trust in the process
        slice::from_raw_parts_mut(buffer.as_mut_ptr(), *primary.size / mem::size_of::<u32>())
    };
    buffer.fill(0);

    let fb = allocate_info_space(*primary.size);
    let buffer_page_start = Page::containing(allocate_info_space(*primary.size));
    let pg_count = primary.size.next_multiple_of(0x1000) / 0x1000;
    let fb_frame_start = Frame::containing(primary.base);
    let fb_page_start = Page::containing(fb);

    for i in 0..pg_count {
        map(
            root_map,
            allocator,
            fb_frame_start + i,
            fb_page_start + i,
            true,
            false,
        );
        map(
            root_map,
            allocator,
            buffer_frame_start + i,
            buffer_page_start + i,
            true,
            false,
        );
    }
    let fb = unsafe {
        // SAFETY: trust in the process
        slice::from_raw_parts_mut(fb.as_mut_ptr(), *primary.size / mem::size_of::<u32>())
    };

    FramebufferInfo {
        fb,
        width: primary.width,
        height: primary.height,
        stride: primary.stride,
        buffer,
    }
}
