use {
    crate::hart,
    boot_protocol::{BootInfo, kernel_init::KernelInitFn},
    common::collections::smallvec::SmallVecBuf,
    core::{
        arch::asm,
        hint, mem,
        sync::atomic::{AtomicUsize, Ordering},
    },
    elf::Elf,
    x64::mem::addr::{Address, VirtAddr},
};

static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);

/// # Safety
/// Needs to be called after all harts have started or failed to start
pub unsafe fn boot_kernel(
    kernel: &Elf<'static>,
    stacks: &mut SmallVecBuf<VirtAddr>,
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

    AP_REMAINING.store(
        unsafe {
            // SAFETY: Guarenteed by caller
            hart::active_harts()
        },
        Ordering::Relaxed,
    );

    hart::ap_boot(entry_info.ap_entry);
    while AP_REMAINING.load(Ordering::Relaxed) > 0 {
        hint::spin_loop();
    }

    do_jump(bsp_stack.as_usize(), bsp_entry);
}

pub fn ap_boot_kernel(stack: usize, ap_entry: VirtAddr) {
    AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
    do_jump(stack, ap_entry.as_usize());
}

fn do_jump(stack: usize, dest: usize) -> ! {
    unsafe {
        asm!(
            "mov rsp, {stack}",
            "jmp {entry}",
            stack = in(reg) stack,
            entry = in(reg) dest,
            options(noreturn)
        );
    }
}
