use {
    crate::{
        hart,
        kernel::{KernelStackSet, mem::KernelHartInfo},
        lapic_timer,
        segmentation::Gdt,
    },
    boot_protocol::kernel_init::KernelInitFn,
    core::{
        arch::asm,
        hint, mem,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    elf::Elf,
    spinlocks::{mutex::SpinMutex, once::SpinOnce},
    system::{
        hart::HartInfo,
        tss::{DF_IST, NMI_IST, ist_index},
        vmem::PHYSICAL_MAPPING_REGION,
    },
    x64::{
        interrupts::InterruptDescriptorTable,
        lapic::LocalApic,
        mem::{
            addr::{Address, VirtAddr},
            segmentation::{selector::SegmentSelector, task_state::TaskStateSegment},
        },
        msr::kernel_gs::KernelGS,
    },
};

static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);
static KHI: SpinOnce<SpinMutex<KernelHartInfo>> = SpinOnce::new();

/// # Safety
/// Needs to be called after all harts have started or failed to start
pub unsafe fn boot_kernel(kernel: &Elf<'static>, mut khi: KernelHartInfo) -> ! {
    let entry = kernel.entry;
    let entry = entry.as_usize();
    let kernel_init: KernelInitFn = unsafe {
        // SAFETY: sometimes rust sucks
        mem::transmute(entry)
    };

    let bsp_stack_set = khi.kernel_stacks.pop_set();

    KHI.init(move || SpinMutex::new(khi));

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
    let tls_base = khi.tls_base;
    populate_hartinfo(&mut khi, bsp_stack_set, tls_base, 0);

    do_jump(kernel_init as usize, &mut khi);
}

pub fn ap_boot_kernel(stack_set: KernelStackSet, ap_entry: KernelInitFn) {
    let mut khi = extract_khi_entry();
    let osid = AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
    let tls_base = khi.tls_base;
    populate_hartinfo(&mut khi, stack_set, tls_base, osid);

    do_jump(ap_entry as usize, &mut khi);
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
    tls_base: VirtAddr,
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
    let tls_base = khi.tlss.pop().expect("Not enough TLSs");

    ExtractedKernelHartInfo {
        hartinfo,
        gdt: khi.gdt_info.gdt,
        kernel_code_selector,
        kernel_data_selector,
        user_code_selector,
        user_data_selector,
        tss_selector,
        tss_segment,
        tls_base,
    }
}

fn populate_hartinfo(
    khi: &mut ExtractedKernelHartInfo,
    stack_set: KernelStackSet,
    tls_base: VirtAddr,
    osid: usize,
) {
    static PIT_SLEEP_USED: AtomicBool = AtomicBool::new(false);

    let hartinfo = &mut khi.hartinfo;

    let lapic_10ms = lapic_timer::ticks_per_10ms();

    **hartinfo = HartInfo {
        hard_id: LocalApic::id(),
        stack: stack_set.stack.as_usize(),
        df_stack: stack_set.df_stack.as_usize(),
        nmi_stack: stack_set.nmi_stack.as_usize(),
        is_bsp: (osid == 0) as usize,
        osid,
        kernel_code_selector: *khi.kernel_code_selector as usize,
        kernel_data_selector: *khi.kernel_data_selector as usize,
        user_code_selector: *khi.user_code_selector as usize,
        user_data_selector: *khi.user_data_selector as usize,
        tss_selector: *khi.tss_selector as usize,
        tss_segment: khi.tss_segment as *const _ as usize,
        tls_base: tls_base.as_usize(),
        lapic_10ms,
    };
}

fn populate_tss(hartinfo: &HartInfo) {
    let tss = unsafe { &mut *(hartinfo.tss_segment as *mut TaskStateSegment) };

    tss.rsp[0] = VirtAddr::from(hartinfo.stack);
    tss.ist[ist_index(DF_IST)] = VirtAddr::from(hartinfo.df_stack);
    tss.ist[ist_index(NMI_IST)] = VirtAddr::from(hartinfo.nmi_stack);
}

fn do_jump(dest: usize, khi: &mut ExtractedKernelHartInfo) -> ! {
    populate_tss(khi.hartinfo);
    unsafe {
        // SAFETY: GdtInfo's configuration is just flat
        // And TSS was just setup
        khi.gdt.load(
            PHYSICAL_MAPPING_REGION.start(),
            khi.kernel_code_selector,
            khi.kernel_data_selector,
            khi.tss_selector,
        );
    }
    Gdt::clear_gs_fs();
    InterruptDescriptorTable::load_null();

    let gs = KernelGS::new(VirtAddr::from(khi.hartinfo as *const HartInfo));
    gs.write();
    KernelGS::swapgs();
    gs.write();

    unsafe {
        asm! {
            "mov rsp, {stack}",

            // Everything 0, except the reserved bit to 1
            "push 0x2",
            "popfq",

            "xor rbx, rbx",
            "xor rcx, rcx",
            "xor rdx, rdx",
            "xor rsi, rsi",
            "xor rdi, rdi",
            "xor rbp, rbp",
            "xor r8,  r8",
            "xor r9,  r9",
            "xor r10, r10",
            "xor r11, r11",
            "xor r12, r12",
            "xor r13, r13",
            "xor r14, r14",
            "xor r15, r15",

            "jmp rax",
            stack = in(reg) khi.hartinfo.stack,
            in("rax") dest,
            options(noreturn)
        };
    }
}
