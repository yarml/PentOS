use {
    core::sync::atomic::{AtomicBool, Ordering},
    spinlocks::once::SpinOnce,
    uefi::{
        Identify,
        boot::{self, SearchType},
        proto::loaded_image::LoadedImage,
    },
    x64::mem::{
        MemorySize,
        addr::{Address, PhysAddr},
    },
};

static IMAGE_BASE: SpinOnce<PhysAddr> = SpinOnce::new();
static IMAGE_SIZE: SpinOnce<MemorySize> = SpinOnce::new();

pub fn init() {
    static LOADED: AtomicBool = AtomicBool::new(false);

    if LOADED.fetch_or(true, Ordering::Relaxed) {
        panic!("Initiliazing bootloader twice!");
    }

    let loaded_image_handle =
        *boot::locate_handle_buffer(SearchType::ByProtocol(&LoadedImage::GUID))
            .expect("Failed to locate LoadedImage protocol")
            .first()
            .expect("No LoadedImage protocol found");
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(loaded_image_handle)
        .expect("Failed to open LoadedImage protocol");

    let (base, size) = loaded_image.info();

    IMAGE_BASE.init(|| PhysAddr::new(base as usize).unwrap());
    IMAGE_SIZE.init(|| MemorySize::new(size as usize));
}

pub fn base() -> PhysAddr {
    *IMAGE_BASE.poll().expect("Loader not initialized")
}
