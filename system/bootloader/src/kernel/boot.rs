use {
    crate::{hart, kernel::mem::KernelHartInfo},
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
        mem::addr::{Address, VirtAddr},
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

    let mut khi = KHI.wait().lock();
    let hartinfo = khi.hartinfos.pop().expect("Not enough HartInfos");
    drop(khi);

    let hartinfo = unsafe {
        // SAFETY: Guarenteed by KHI invariants
        &mut *hartinfo.as_mut_ptr::<HartInfo>()
    };

    hartinfo.hard_id = lapic::id_cpuid();
    hartinfo.stack = bsp_stack.as_usize();
    hartinfo.is_bsp = true;
    hartinfo.osid = 0;

    do_jump(kernel_init as usize, hartinfo);
}

pub fn ap_boot_kernel(stack: usize, ap_entry: KernelInitFn) {
    let khi_mutex = KHI.wait();

    let mut khi = khi_mutex.lock();
    let hartinfo = khi.hartinfos.pop().expect("Not enough HartInfos");
    drop(khi);

    let hartinfo = unsafe {
        // SAFETY: Guarenteed by KHI invariants
        &mut *hartinfo.as_mut_ptr::<HartInfo>()
    };

    hartinfo.hard_id = lapic::id_cpuid();
    hartinfo.stack = stack;
    hartinfo.is_bsp = false;
    hartinfo.osid = AP_REMAINING.fetch_sub(1, Ordering::Relaxed);

    do_jump(ap_entry as usize, hartinfo);
}

fn do_jump(dest: usize, hartinfo: &HartInfo) -> ! {
    let gs = KernelGS::new(VirtAddr::from(hartinfo as *const HartInfo));
    gs.write();
    KernelGS::swapgs();
    gs.write();

    unsafe {
        asm!(
            "mov rsp, {stack}",
            "jmp {entry}",
            stack = in(reg) hartinfo.stack,
            entry = in(reg) dest,
            options(noreturn)
        );
    }
}
