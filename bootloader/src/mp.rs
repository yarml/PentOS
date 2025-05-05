use crate::phys_mmap::PhysMemMap;
use crate::pit;
use crate::topology;
use config::topology::hart::MAX_AP_RETRIES;
use core::ffi;
use core::hint;
use core::slice;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use x64::lapic;
use x64::lapic::IPIDeliveryMode;
use x64::lapic::IPIDestination;
use x64::lapic::IPIDestinationMode;
use x64::lapic::IPILevel;
use x64::lapic::IPITriggerMode;
use x64::lapic::InterProcessorInterrupt;
use x64::lapic::LocalApicPointer;
use x64::mem::MemorySize;
use x64::mem::PhysicalMemoryRegion;
use x64::mem::addr::Address;
use x64::mem::addr::PhysAddr;

unsafe extern "C" {
    static ap_bootstrap_begin: ffi::c_void;
    static ap_bootstrap_end: ffi::c_void;
}

const MAX_AP_CODE_SIZE: usize = 1024;
const AP_ALIVE_FLAG_OFFSET: usize = 1024;

pub fn init(legacy_mmap: PhysMemMap<16>) {
    // First sanity check, is 0 a valid address
    if !legacy_mmap
        .iter()
        .any(|entry| entry.contains(PhysAddr::null()))
    {
        panic!("Frame 0 is invalid");
    }

    // Next, find a scratch 64k segment that will be used to bootstrap processors
    let Some(chunk) = legacy_mmap
        .iter()
        .flat_map(|entry| entry.chunks(MemorySize::new(64 * 1024), MemorySize::new(64 * 1024)))
        // Avoid first frame.
        .find(|chunk| *chunk.start() != 0)
    else {
        panic!("Could not find chunk to load AP cores");
    };

    // Load ap bootsrap codde into the chosen chunk
    let ap_bootstrap_begin_loc = unsafe {
        // # Safety
        // Nothing to worry about
        &ap_bootstrap_begin
    } as *const _ as usize;
    let ap_bootstrap_end_loc = unsafe {
        // # Safety
        // Nothing to worry about
        &ap_bootstrap_end
    } as *const _ as usize;
    let ap_bootstrap_size = ap_bootstrap_end_loc - ap_bootstrap_begin_loc;

    if ap_bootstrap_size > MAX_AP_CODE_SIZE {
        hint::cold_path();
        panic!("AP bootstrap code too large");
    }

    let ap_bootstrap_code = unsafe {
        // # Safety
        // Nothing to worry about
        slice::from_raw_parts(ap_bootstrap_begin_loc as *const u8, ap_bootstrap_size)
    };
    let ap_bootstrap_destination = unsafe {
        // # Safety
        // We own legacy memory which chunk is part of
        slice::from_raw_parts_mut(chunk.start().as_mut_ptr::<u8>(), ap_bootstrap_size)
    };

    ap_bootstrap_destination.copy_from_slice(ap_bootstrap_code);

    // Load IVT of interrupt 0x20 with the chosen chunk
    let ivt_entry = &mut unsafe {
        // # Safety
        // We own legacy memory range in the bootloader phase
        *((0x20 * 4) as *mut u32)
    };
    *ivt_entry = chunk.start().as_u64() as u32;

    let bspid = lapic::id_cpuid();
    let lapic = todo!();

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
        delivery_mode: IPIDeliveryMode::StartUp { vector: 0x20 },
        destination: IPIDestination::Explicit {
            tartget_apicid: apic_id,
        },
        destination_mode: IPIDestinationMode::Physical,
    };

    let alive_flag = unsafe {
        // # Safety
        // We own chunk memory
        (chunk.start() + AP_ALIVE_FLAG_OFFSET).to_ref::<AtomicU64>()
    };
    alive_flag.store(0, Ordering::Relaxed);
    lapic.send_ipi(init_ipi);
    // Linux does not put any delay here for post ~2000 processors, neither do I
    lapic.send_ipi(init_deassert_ipi);

    let success = 'success: {
        for attempt in 0..MAX_AP_RETRIES {
            lapic.send_ipi(startup_ipi);

            // Linux does only 10us, me just follow, but me want to becreative, so me make it exponential
            pit::sleep_us(10 * (100 * attempt + 1));

            if alive_flag.load(Ordering::Relaxed) != 0 {
                break 'success true;
            }
        }
        false
    };

    if !success {
        panic!("Could not start processor after {MAX_AP_RETRIES}: {apic_id}");
    }
}
