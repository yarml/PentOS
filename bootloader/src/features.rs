use core::arch::x86_64::__cpuid;
use core::arch::x86_64::__cpuid_count;
use core::arch::x86_64::CpuidResult;
use core::ffi::c_void;
use core::hint;
use core::ptr::null_mut;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use boot_protocol::features::FeatureSet;
use boot_protocol::features::Vendor;
use spinlocks::once::Once;
use uefi::Identify;
use uefi::boot;
use uefi::boot::SearchType;
use uefi::proto::pi::mp::MpServices;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FeatureDetect {
    Sufficient(FeatureSet),
    Insufficient(InsufficientReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsufficientReason {
    UnsupportedVendor([u8; 12]),
    UnreachableCpuidLeaf(usize),
    UnreachableCpuidExtendedLeaf(usize),
    NoHugePages,
    NoApic,
    NoSyscall,
    NoPAT,
    NoExecDisable,
    NoLongMode, // Can we even get here???
}

impl FeatureDetect {
    pub fn detect() -> Self {
        let CpuidResult {
            eax: max_basic,
            ebx: vendor0,
            edx: vendor1,
            ecx: vendor2,
        } = unsafe { __cpuid(0) };
        let max_basic = max_basic as usize;
        let max_extended = unsafe { __cpuid(0x8000_0000) }.eax as usize;

        let vendor = match Vendor::try_from({
            let mut vendor = [0; 12];
            vendor[0..4].copy_from_slice(&vendor0.to_ne_bytes());
            vendor[4..8].copy_from_slice(&vendor1.to_ne_bytes());
            vendor[8..12].copy_from_slice(&vendor2.to_ne_bytes());
            vendor
        }) {
            Ok(vendor) => vendor,
            Err(unsupported) => {
                return Self::Insufficient(InsufficientReason::UnsupportedVendor(unsupported));
            }
        };
        vendor_detect(vendor, max_basic, max_extended)
    }
}

fn vendor_detect(vendor: Vendor, max_basic: usize, max_extended: usize) -> FeatureDetect {
    let detector = match vendor {
        Vendor::GenuineIntel | Vendor::AuthenticAMD => intel_amd_detect,
    };
    detector(vendor, max_basic, max_extended)
}

fn intel_amd_detect(vendor: Vendor, max_basic: usize, max_extended: usize) -> FeatureDetect {
    const REQ_MAX_BASIC: usize = 1;
    const REQ_MAX_EXT: usize = 0x8000_0001;

    if max_basic < REQ_MAX_BASIC {
        return FeatureDetect::Insufficient(InsufficientReason::UnreachableCpuidLeaf(
            REQ_MAX_BASIC,
        ));
    }
    if max_extended < REQ_MAX_EXT {
        return FeatureDetect::Insufficient(InsufficientReason::UnreachableCpuidExtendedLeaf(
            REQ_MAX_EXT,
        ));
    }

    let cpuid1 = unsafe { __cpuid(1) };
    let cpuidext1 = unsafe { __cpuid(0x8000_0001) };

    let has_huge_pages = (cpuidext1.edx >> 26) & 1 == 1;
    let has_apic = (cpuid1.edx >> 9) & 1 == 1;
    let has_syscall = (cpuidext1.edx >> 11) & 1 == 1;
    let has_pat = (cpuid1.edx >> 16) & 1 == 1;
    let has_noexec = (cpuidext1.edx >> 20) & 1 == 1;
    let has_long_mode = (cpuidext1.edx >> 29) & 1 == 1;

    if !has_huge_pages {
        return FeatureDetect::Insufficient(InsufficientReason::NoHugePages);
    }
    if !has_apic {
        return FeatureDetect::Insufficient(InsufficientReason::NoApic);
    }
    if !has_syscall {
        return FeatureDetect::Insufficient(InsufficientReason::NoSyscall);
    }
    if !has_pat {
        return FeatureDetect::Insufficient(InsufficientReason::NoPAT);
    }
    if !has_noexec {
        return FeatureDetect::Insufficient(InsufficientReason::NoExecDisable);
    }
    if !has_long_mode {
        return FeatureDetect::Insufficient(InsufficientReason::NoLongMode);
    }

    // I couldn't find in AMD manual where PCID functionality is
    // However they do mention in APM Volume 2 Section 5.5.1 page 158 (March 2024)
    // that it can be found with CPUID 01.ECX[PCID], without mentioning the bit
    // However APM Volume 3 Appendix E pages 602-603 (March 2024) do not mention any PCID bit
    // in ECX for CPUID 01. Assuming they are just like Intel where
    // PCID is CPUID 01.ECX[bit 17], even though it is marked as reserved in the
    // Aforementioned APM Volume 3
    let context_id = (cpuid1.ecx >> 17) & 1 == 1;

    // Yet strangely, AMD talks about INVPCID without ambiguity...
    let (inv_context_id, shadow_stack, pk_user, pk_super) = if max_basic >= 7 {
        let cpuid7_0 = unsafe { __cpuid_count(7, 0) };
        let inv_context_id = (cpuid7_0.ebx >> 10) & 1 == 1;
        let shadow_stack = (cpuid7_0.ecx >> 7) & 1 == 1;
        let pk_user = (cpuid7_0.ecx >> 3) & 1 == 1;
        let pk_super = match vendor {
            Vendor::GenuineIntel => (cpuid7_0.ecx >> 31) & 1 == 1,
            Vendor::AuthenticAMD => false, // I couldn't find any mention of PKS, and bit 31 is marked as reserved
        };
        (inv_context_id, shadow_stack, pk_user, pk_super)
    } else {
        (false, false, false, false)
    };

    FeatureDetect::Sufficient(FeatureSet {
        vendor,
        context_id,
        inv_context_id,
        shadow_stack,
        pk_user,
        pk_super,
    })
}

static BSP_FEATURES: Once<FeatureSet> = Once::new();
static AP_SYMMETRIC: AtomicBool = AtomicBool::new(true);
static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);

pub fn featureset() -> FeatureSet {
    let features_detect = FeatureDetect::detect();
    let features = match features_detect {
        FeatureDetect::Sufficient(features) => features,
        FeatureDetect::Insufficient(reason) => {
            panic!("Insufficient features: {:?}", reason);
        }
    };

    BSP_FEATURES.init(|| features);

    let mp_handle = *boot::locate_handle_buffer(SearchType::ByProtocol(&MpServices::GUID))
        .expect("Failed to locate MP Services protocol")
        .first()
        .expect("No MP Services protocol found");
    let mp = boot::open_protocol_exclusive::<MpServices>(mp_handle)
        .expect("Failed to open MP Services protocol");
    let numproc = mp
        .get_number_of_processors()
        .expect("Failed to get number of processors");
    AP_REMAINING.store(numproc.enabled - 1, Ordering::Relaxed);

    mp.startup_all_aps(false, ap_feature_detect, null_mut(), None, None)
        .expect("Failed to start APs for feature detection");

    while AP_REMAINING.load(Ordering::Relaxed) > 0 {
        hint::spin_loop();
    }

    if !AP_SYMMETRIC.load(Ordering::Relaxed) {
        todo!("APs have different features");
    }

    features
}

extern "efiapi" fn ap_feature_detect(_: *mut c_void) {
    let bsp_features = *BSP_FEATURES.get().unwrap();
    let my_features = FeatureDetect::detect();
    if my_features != FeatureDetect::Sufficient(bsp_features) {
        AP_SYMMETRIC.store(false, Ordering::Relaxed);
    }
    AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
}
