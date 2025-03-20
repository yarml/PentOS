use core::sync::atomic::AtomicU8;
use core::sync::atomic::Ordering;

static BOOT_STAGE: AtomicU8 = AtomicU8::new(BootStage::PreBoot as u8);

#[repr(u8)]
pub enum BootStage {
    PreBoot,
    PostBoot,
}

pub fn set_postboot() {
    BOOT_STAGE.store(BootStage::PostBoot as u8, Ordering::Relaxed);
}

pub fn is_preboot() -> bool {
    BOOT_STAGE.load(Ordering::Relaxed) == BootStage::PreBoot as u8
}

pub fn is_postboot() -> bool {
    BOOT_STAGE.load(Ordering::Relaxed) == BootStage::PostBoot as u8
}
