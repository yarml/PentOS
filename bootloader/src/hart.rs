use {
    crate::{phys_mmap::PhysMemMap, pit, topology},
    config::{topology::hart::MAX_AP_RETRIES, vmem::LOCAL_APIC_REGION},
    core::{
        hint, slice,
        sync::atomic::{AtomicU8, AtomicU32, Ordering},
    },
    x64::{
        lapic::{
            self, IPIDeliveryMode, IPIDestination, IPIDestinationMode, IPILevel, IPITriggerMode,
            InterProcessorInterrupt, LocalApicPointer,
        },
        mem::{
            MemorySize, PhysicalMemoryRegion,
            addr::Address,
            frame::size::{Frame4KiB, FrameSize},
        },
    },
};

const MAX_AP_CODE_SIZE: usize = 1024;

static AP_INIT_CODE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ap_init.bin"));
const _: () = assert!(
    AP_INIT_CODE.len() <= MAX_AP_CODE_SIZE,
    "AP init code too large"
);

// Sync with bootloader/src/hart/ap_init.asm
const BASE_OFFSET: usize = 1028;
const STATUS_FLAG_OFFSET: usize = 1024;

const STATUS_WAIT: u8 = 0;
const STATUS_ALIVE: u8 = 1;
const STATUS_DONE: u8 = 2;
const STATUS_ERROR: u8 = 3;

/// # Safety
/// Must guarentee that IA32_APIC_BASE is mapped up to 4KiB to config/vmem:LOCAL_APIC_REGION
/// And that legacy memory is memory mapped
pub unsafe fn init(legacy_mmap: PhysMemMap<16>) {
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
        wakeup_hart(lapic, hart.apic_id as u8, chunk);
    }
}

fn wakeup_hart(lapic: LocalApicPointer, apic_id: u8, chunk: PhysicalMemoryRegion) {
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
        (chunk.start() + STATUS_FLAG_OFFSET).to_ref::<AtomicU8>()
    };
    let base = unsafe {
        // SAFETY: We own chunk memory
        (chunk.start() + BASE_OFFSET).to_ref::<AtomicU32>()
    };
    status_flag.store(STATUS_WAIT, Ordering::Relaxed);
    base.store(chunk.start().as_usize() as u32, Ordering::Relaxed);

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
        panic!("Could not start processor after {MAX_AP_RETRIES} attempts: {apic_id}");
    }

    while status_flag.load(Ordering::Relaxed) == STATUS_ALIVE {
        hint::spin_loop();
    }

    if status_flag.load(Ordering::Relaxed) != STATUS_DONE {
        panic!("AP failed initializing: {apic_id}");
    }
}
