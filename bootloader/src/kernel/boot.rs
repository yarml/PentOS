use {
    boot_protocol::{BootInfo, STACK_SIZE, kernel_init::KernelInitFn},
    core::{
        arch::asm,
        hint, mem,
        sync::atomic::{AtomicUsize, Ordering},
    },
    elf::Elf,
    spinlocks::once::Once,
    x64::{
        lapic,
        mem::addr::{Address, VirtAddr},
    },
};

struct ApInfo {
    pub ap_entry: VirtAddr,
    pub stack_base: VirtAddr,
}

static AP_CEDE: Once<ApInfo> = Once::new();
static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);

pub fn bsp_cede_control(kernel: &Elf<'static>, stack: VirtAddr, bootinfo: &BootInfo) -> ! {
    let entry = kernel.entry;
    let entry = entry.as_usize();
    let kernel_init: KernelInitFn = unsafe {
        // SAFETY: sometimes rust sucks
        mem::transmute(entry)
    };
    let entry_info = kernel_init(bootinfo);

    let bsp_entry = entry_info.bsp_entry.as_usize();

    AP_CEDE.init(|| ApInfo {
        ap_entry: entry_info.ap_entry,
        stack_base: stack,
    });
    while AP_REMAINING.load(Ordering::Relaxed) > 0 {
        hint::spin_loop();
    }

    let stack = stack.as_usize();

    do_jump(stack, bsp_entry);
}

fn ap_cede_control() {
    AP_REMAINING.fetch_add(1, Ordering::Relaxed);
    while AP_CEDE.get().is_none() {
        hint::spin_loop();
    }

    let ap_info = AP_CEDE.get().unwrap();

    let ap_entry = ap_info.ap_entry.as_usize();
    let stack = ap_info.stack_base.as_usize() + STACK_SIZE * lapic::id_cpuid();

    AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
    do_jump(stack, ap_entry);
}

#[allow(unreachable_code, unused_variables)]
fn do_jump(stack: usize, dest: usize) -> ! {
    // loop {
    //     hint::spin_loop();
    // }
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
