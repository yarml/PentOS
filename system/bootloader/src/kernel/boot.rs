use {
    crate::{hart, kernel::mem::KernelHartInfo, segmentation::Gdt},
    boot_protocol::kernel_init::KernelInitFn,
    core::{
        arch::asm,
        hint, mem,
        sync::atomic::{AtomicUsize, Ordering},
    },
    elf::Elf,
    spinlocks::{mutex::Mutex, once::Once},
    system::hart::HartInfo,
    x64::{
        lapic,
        mem::{
            addr::{Address, VirtAddr},
            segmentation::{selector::SegmentSelector, task_state::TaskStateSegment},
        },
        msr::kernel_gs::KernelGS,
    },
};

static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);
static KHI: Once<Mutex<KernelHartInfo>> = Once::new();

/// # Safety
/// Needs to be called after all harts have started or failed to start
pub unsafe fn boot_kernel(kernel: &Elf<'static>, mut khi: KernelHartInfo) -> ! {
    let entry = kernel.entry;
    let entry = entry.as_usize();
    let kernel_init: KernelInitFn = unsafe {
        // SAFETY: sometimes rust sucks
        mem::transmute(entry)
    };

    let bsp_stack = khi.stacks.pop().expect("Did not find stack for BSP");

    for hartinfo in &mut khi.hartinfos {
        let hartinfo = unsafe {
            // SAFETY: Guarenteed by KHI invariants
            &mut *hartinfo.as_mut_ptr::<HartInfo>()
        };
        hartinfo.tls_base = khi.tlss.pop().expect("Not enough TLSs").as_usize();
    }

    KHI.init(move || Mutex::new(khi));

    AP_REMAINING.store(
        unsafe {
            // SAFETY: Guarenteed by caller
            hart::active_harts() - 1 // we don't wait for the BSP... We are the BSP
        },
        Ordering::Relaxed,
    );

    hart::ap_boot(kernel_init);
    while AP_REMAINING.load(Ordering::Relaxed) > 0 {
        hint::spin_loop();
    }

    let mut khi = extract_khi_entry();
    populate_hartinfo(&mut khi, bsp_stack.as_usize(), 0);

    do_jump(kernel_init as usize, &mut khi);
}

pub fn ap_boot_kernel(stack: usize, ap_entry: KernelInitFn) {
    let mut khi = extract_khi_entry();
    let osid = AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
    populate_hartinfo(&mut khi, stack, osid);

    do_jump(ap_entry as usize, &mut khi);
}

fn do_jump(dest: usize, khi: &mut ExtractedKernelHartInfo) -> ! {
    let gs = KernelGS::new(VirtAddr::from(khi.hartinfo as *const HartInfo));
    gs.write();
    KernelGS::swapgs();
    gs.write();

    populate_tss(khi.tss_segment, khi.hartinfo.stack);

    unsafe {
        // SAFETY: GdtInfo's configuration is just flat
        // And TSS was just setup
        khi.gdt.load(
            khi.kernel_code_selector,
            khi.kernel_data_selector,
            khi.tss_selector,
        );
    }

    unsafe {
        asm!(
            "mov rsp, {stack}",
            "jmp {entry}",
            stack = in(reg) khi.hartinfo.stack,
            entry = in(reg) dest,
            options(noreturn)
        );
    }
}

struct ExtractedKernelHartInfo {
    hartinfo: &'static mut HartInfo,
    gdt: &'static Gdt,
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    tss_segment: &'static mut TaskStateSegment,
}

fn extract_khi_entry() -> ExtractedKernelHartInfo {
    let khi_mutex = KHI.wait();
    let mut khi = khi_mutex.lock();
    let hartinfo = unsafe {
        // SAFETY: Guarenteed by KHI invariants
        &mut *khi
            .hartinfos
            .pop()
            .expect("Not enough HartInfos")
            .as_mut_ptr::<HartInfo>()
    };
    let kernel_code_selector = khi.gdt_info.kernel_code;
    let kernel_data_selector = khi.gdt_info.kernel_data;
    let user_code_selector = khi.gdt_info.user_code;
    let user_data_selector = khi.gdt_info.user_data;
    let tss_data = khi
        .gdt_info
        .tss_table
        .pop()
        .expect("Not enough TSS selectors");
    let tss_selector = tss_data.0;
    let tss_segment = unsafe {
        // SAFETY: exists within offset memory
        tss_data.1.to_mut()
    };
    ExtractedKernelHartInfo {
        hartinfo,
        gdt: khi.gdt_info.gdt,
        kernel_code_selector,
        kernel_data_selector,
        user_code_selector,
        user_data_selector,
        tss_selector,
        tss_segment,
    }
}

fn populate_hartinfo(khi: &mut ExtractedKernelHartInfo, stack: usize, osid: usize) {
    let hartinfo = &mut khi.hartinfo;

    hartinfo.hard_id = lapic::id_cpuid();
    hartinfo.stack = stack;
    hartinfo.is_bsp = (osid == 0) as usize;
    hartinfo.osid = osid;

    hartinfo.kernel_code_selector = *khi.kernel_code_selector as usize;
    hartinfo.kernel_data_selector = *khi.kernel_data_selector as usize;
    hartinfo.user_code_selector = *khi.user_code_selector as usize;
    hartinfo.user_data_selector = *khi.user_data_selector as usize;
    hartinfo.tss_selector = *khi.tss_selector as usize;
}

fn populate_tss(tss: &mut TaskStateSegment, stack: usize) {
    tss.rsp[0] = VirtAddr::from(stack);
}
