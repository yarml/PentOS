use {
    crate::{
        features::{self, FeatureDetect},
        kernel,
        phys_mmap::PhysMemMap,
        pit, topology,
    },
    boot_protocol::kernel_init::KernelInitFn,
    config::{topology::hart::MAX_AP_RETRIES, vmem::LOCAL_APIC_REGION},
    core::{
        hint, slice,
        sync::atomic::{AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering},
    },
    log::{debug, error},
    spinlocks::once::Once,
    utils::collections::smallvec::SmallVecBuf,
    x64::{
        control::{CR0, CR4},
        lapic::{
            self, IPIDeliveryMode, IPIDestination, IPIDestinationMode, IPILevel, IPITriggerMode,
            InterProcessorInterrupt, LocalApicPointer,
        },
        mem::{
            MemorySize, PhysicalMemoryRegion,
            addr::{Address, PhysAddr, VirtAddr},
            frame::{
                Frame,
                size::{Frame4KiB, FrameSize},
            },
            paging::PagingRootEntry,
        },
        msr::{
            apic_base::{ApicBase, STANDARD_PHYS_BASE},
            efer::Efer,
        },
    },
};
static AP_INIT_CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ap_init.bin"));
const _: () = assert!(
    AP_INIT_CODE.len() <= MAX_AP_CODE_SIZE,
    "AP init code too large"
);

const MAX_AP_CODE_SIZE: usize = 1024;

// sync with bootloader/src/hart/ap_init.asm
const BASE_OFFSET: usize = 1028;
const STATUS_FLAG_OFFSET: usize = 1024;
const CR3_OFFSET: usize = 1032;
const ENTRYPOINT_OFFSET: usize = 1040;
const STACK_OFFSET: usize = 1048;

const STATUS_WAIT: u8 = 0;
const STATUS_ALIVE: u8 = 1;
const STATUS_DONE: u8 = 2;
const STATUS_ERROR: u8 = 3;
// end sync

/// Counts actually working and initialized harts.
static HART_ACTIVE: AtomicUsize = AtomicUsize::new(1); // BSP already in

static AP_BOOT_ENTRYPOINT: Once<KernelInitFn> = Once::new();

/// # Safety
/// Must guarentee that IA32_APIC_BASE is mapped up to 4KiB to config/vmem:LOCAL_APIC_REGION
/// And that legacy memory is memory mapped
/// And that kernel stacks are mapped
pub unsafe fn init(
    legacy_mmap: PhysMemMap<16>,
    map_root: PagingRootEntry,
    stacks: &mut SmallVecBuf<VirtAddr>,
) {
    // find a scratch 64k segment that will be used to bootstrap processors
    let Some(chunk) = legacy_mmap
        .iter()
        .flat_map(|entry| entry.chunks(MemorySize::new(64 * 1024), MemorySize::new(64 * 1024)))
        // Avoid first frame.
        .find(|chunk| *chunk.start() != 0)
    else {
        panic!("Could not find chunk to load AP cores");
    };

    let ap_bootstrap_destination = unsafe {
        // SAFETY: We own legacy memory which chunk is part of
        slice::from_raw_parts_mut(chunk.start().as_mut_ptr::<u8>(), AP_INIT_CODE.len())
    };

    ap_bootstrap_destination.copy_from_slice(AP_INIT_CODE);

    let bspid = lapic::id_cpuid();
    let lapic = unsafe {
        // SAFETY: Guarenteed by caller
        LocalApicPointer::from_virt_addr(LOCAL_APIC_REGION.start())
    };

    let topology = topology::topology();
    for hart in topology.harts.iter().filter(|hart| hart.apic_id != bspid) {
        let stack = stacks.pop().expect("Did not find stack for AP");
        wakeup_hart(lapic, hart.apic_id as u8, chunk, map_root, stack);
    }
}

fn wakeup_hart(
    lapic: LocalApicPointer,
    apic_id: u8,
    chunk: PhysicalMemoryRegion,
    map_root: PagingRootEntry,
    ap_stack: VirtAddr,
) {
    let init_ipi = InterProcessorInterrupt {
        delivery_mode: IPIDeliveryMode::Init {
            level: IPILevel::Assert,
        },
        destination: IPIDestination::Explicit {
            tartget_apicid: apic_id,
        },
        destination_mode: IPIDestinationMode::Physical,
    };
    let init_deassert_ipi = InterProcessorInterrupt {
        delivery_mode: IPIDeliveryMode::Init {
            level: IPILevel::Deassert {
                trigger: IPITriggerMode::Level,
            },
        },
        destination: IPIDestination::Explicit {
            tartget_apicid: apic_id,
        },
        destination_mode: IPIDestinationMode::Physical,
    };
    let startup_ipi = InterProcessorInterrupt {
        delivery_mode: IPIDeliveryMode::StartUp {
            vector: (chunk.start().as_usize() >> Frame4KiB::SHIFT) as u8,
        },
        destination: IPIDestination::Explicit {
            tartget_apicid: apic_id,
        },
        destination_mode: IPIDestinationMode::Physical,
    };

    let status_flag = unsafe {
        // SAFETY: We own chunk memory
        (chunk.start() + STATUS_FLAG_OFFSET).to_ref_ident::<AtomicU8>()
    };
    let base = unsafe {
        // SAFETY: We own chunk memory
        (chunk.start() + BASE_OFFSET).to_ref_ident::<AtomicU32>()
    };
    let cr3val = unsafe {
        // SAFETY: We own chunk memory
        (chunk.start() + CR3_OFFSET).to_ref_ident::<AtomicU32>()
    };
    let entrypoint = unsafe {
        // SAFETY: We own chunk memory
        (chunk.start() + ENTRYPOINT_OFFSET).to_ref_ident::<AtomicU64>()
    };
    let stack = unsafe {
        // SAFETY: We own chunk memory
        (chunk.start() + STACK_OFFSET).to_ref_ident::<AtomicU64>()
    };

    status_flag.store(STATUS_WAIT, Ordering::Relaxed);
    base.store(chunk.start().as_usize() as u32, Ordering::Relaxed);
    cr3val.store(map_root.rawval() as u32, Ordering::Relaxed);
    entrypoint.store(ap_entrypoint as *const () as u64, Ordering::Relaxed);
    stack.store(ap_stack.as_u64(), Ordering::Relaxed);

    lapic.send_ipi(init_ipi);
    // Linux does not put any delay here for post ~2000 processors, neither do I
    lapic.send_ipi(init_deassert_ipi);

    let success = 'success: {
        for attempt in 0..MAX_AP_RETRIES {
            lapic.send_ipi(startup_ipi);
            // Linux does only 10us, me just follow, but me want to be creative, so me make it exponential
            // but cap it at 50ms
            pit::sleep_us(usize::min(10 * (100 * attempt + 1), 50 * 1000));

            if status_flag.load(Ordering::Relaxed) != STATUS_WAIT {
                break 'success true;
            }
        }
        false
    };

    if !success {
        error!("Could not start processor after {MAX_AP_RETRIES} attempts: {apic_id}");
    }

    while status_flag.load(Ordering::Relaxed) == STATUS_ALIVE {
        hint::spin_loop();
    }

    if status_flag.load(Ordering::Relaxed) != STATUS_DONE {
        error!("AP failed initializing: {apic_id}");
    }
}

/// # Safety
/// Needs to be called after all harts have booted, or failed to boot
pub unsafe fn active_harts() -> usize {
    HART_ACTIVE.load(Ordering::Relaxed)
}

pub fn ap_boot(entrypoint: KernelInitFn) {
    AP_BOOT_ENTRYPOINT.init(|| entrypoint);
}

extern "sysv64" fn ap_entrypoint(base: usize, stack: usize) {
    // APs arrive one at a time here
    // Here we can finally take a breathe and use nice APIs
    // to continue the setup into a deterministic state
    // We are using the same page table entries as the BSP at this point
    // Both identity mapping and offset mapping are active

    let status_flag = unsafe {
        // SAFETY: We own chunk memory
        PhysAddr::new_panic(base + STATUS_FLAG_OFFSET).to_ref_ident::<AtomicU8>()
    };

    let FeatureDetect::Sufficient(features) = FeatureDetect::detect() else {
        status_flag.store(STATUS_ERROR, Ordering::Relaxed);
        panic!("AP insufficient features");
    };

    if features::bsp_features() != features {
        status_flag.store(STATUS_ERROR, Ordering::Relaxed);
        panic!("Assymetric AP");
    }
    status_flag.store(STATUS_DONE, Ordering::Relaxed);
    HART_ACTIVE.fetch_add(1, Ordering::Relaxed);

    debug!("AP core UP!");

    known_state();

    let ap_entry = *AP_BOOT_ENTRYPOINT.wait();
    kernel::ap_boot_kernel(stack, ap_entry);
}

/// Put CPU in a known state
pub fn known_state() {
    unsafe {
        // SAFETY: These are normally safe states
        Efer::new().exec_disable(true).syscall(true).write();
        ApicBase::read()
            .with_enabled(true)
            .with_phys_base(Frame::containing(STANDARD_PHYS_BASE))
            .write();
        CR0::new().numeric_error(true).write_protect(true).write();
        CR4::new()
            .global_pages(true)
            .fsgsbase(true)
            .debug_extensions(true)
            .write();
    }
}
