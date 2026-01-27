use {
    crate::{
        allocator::{ALLOCATOR_CAP, PostBootAllocator},
        virt_mmap::map,
    },
    boot_protocol::framebuffer::FramebufferInfo,
    config::vmem::{FRAME_DOUBLEBUFFER_REGION, FRAMEBUFFER_REGION},
    core::{mem, slice},
    uefi::{
        Identify,
        boot::{self, SearchType},
        proto::console::gop::{GraphicsOutput, Mode, PixelFormat},
    },
    x64::{
        framebuffer::PixelMode,
        mem::{
            MemorySize,
            addr::{Address, PhysAddr},
            frame::{Frame, size::Frame4KiB},
            page::{Page, size::Page4KiB},
            paging::PagingRootEntry,
        },
        msr::pat::MemoryType,
    },
};

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
                best_area > this_area
            }) {
                Some(this_mode)
            } else {
                best_mode
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
    let base = PhysAddr::new_panic(fb.as_mut_ptr() as usize);
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
    let buffer_frame_start = Frame::<Frame4KiB>::containing(buffer);

    let bufferptr = buffer.as_mut_ptr();
    let bufferlen = *primary.size / mem::size_of::<u32>();

    let buffer = unsafe {
        // SAFETY: trust in the process
        slice::from_raw_parts_mut(bufferptr, bufferlen)
    };
    buffer.fill(0);

    let fb = FRAMEBUFFER_REGION.start();
    let buffer_page_start = Page::<Page4KiB>::containing(FRAME_DOUBLEBUFFER_REGION.start());

    let pg_count = primary.size.next_multiple_of(0x1000) / 0x1000;
    let fb_frame_start = Frame::<Frame4KiB>::containing(primary.base);
    let fb_page_start = Page::<Page4KiB>::containing(fb);

    for i in 0..pg_count {
        map(
            root_map,
            allocator,
            fb_frame_start + i,
            fb_page_start + i,
            true,
            false,
            MemoryType::WriteCombining,
        );
        map(
            root_map,
            allocator,
            buffer_frame_start + i,
            buffer_page_start + i,
            true,
            false,
            MemoryType::WriteBack,
        );
    }

    let fbptr = fb.as_mut_ptr();
    let fblen = *primary.size / mem::size_of::<u32>();

    FramebufferInfo {
        fbptr,
        fblen,
        width: primary.width,
        height: primary.height,
        stride: primary.stride,
        bufferptr,
        bufferlen,
    }
}
