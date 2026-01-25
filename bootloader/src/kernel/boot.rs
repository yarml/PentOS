use {
    boot_protocol::{BootInfo, kernel_init::KernelInitFn},
    common::collections::smallvec::SmallVec,
    config::topology::hart::MAX_HART_COUNT,
    core::{
        arch::asm,
        hint, mem,
        sync::atomic::{AtomicUsize, Ordering},
    },
    elf::Elf,
    spinlocks::mutex::Mutex,
    x64::mem::addr::{Address, VirtAddr},
};

struct ApInfo {
    pub ap_entry: VirtAddr,
    pub stacks: SmallVec<VirtAddr, MAX_HART_COUNT>,
}

static AP_CEDE: Mutex<Option<ApInfo>> = Mutex::new(None);
static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);

pub fn boot_kernel(
    kernel: &Elf<'static>,
    mut stacks: SmallVec<VirtAddr, MAX_HART_COUNT>,
    bootinfo: &BootInfo,
) -> ! {
    let entry = kernel.entry;
    let entry = entry.as_usize();
    let kernel_init: KernelInitFn = unsafe {
        // SAFETY: sometimes rust sucks
        mem::transmute(entry)
    };
    let entry_info = kernel_init(bootinfo);
    let bsp_entry = entry_info.bsp_entry.as_usize();

    let bsp_stack = stacks.pop().expect("Did not find stack for BSP");

    let mut ap_cede = AP_CEDE.lock();
    *ap_cede = Some(ApInfo {
        ap_entry: entry_info.ap_entry,
        stacks,
    });

    // TODO: set AP_REMAINING to hart count, make APs just to ap_boot_kernel

    while AP_REMAINING.load(Ordering::Relaxed) > 0 {
        hint::spin_loop();
    }

    do_jump(bsp_stack.as_usize(), bsp_entry);
}

fn ap_boot_kernel() {
    AP_REMAINING.fetch_add(1, Ordering::Relaxed);
    let mut ap_info = AP_CEDE.lock();
    let Some((ap_entry, stacks)) = ap_info.as_mut().map(|inf| (inf.ap_entry, &mut inf.stacks))
    else {
        panic!("AP_INFO is not initialized");
    };

    let ap_entry = ap_entry.as_usize();
    let ap_stack = stacks.pop().expect("Did not find stack for AP");

    AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
    do_jump(ap_stack.as_usize(), ap_entry);
}

fn do_jump(stack: usize, dest: usize) -> ! {
    unsafe {
        asm!(
            "mov rsp, {0}",
            "jmp {1}",
            in(reg) stack,
            in(reg) dest,
            options(noreturn)
        );
    }
}
