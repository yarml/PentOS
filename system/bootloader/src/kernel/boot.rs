use {
    crate::hart,
    boot_protocol::kernel_init::KernelInitFn,
    core::{
        arch::asm,
        hint, mem,
        sync::atomic::{AtomicUsize, Ordering},
    },
    elf::Elf,
    utils::collections::smallvec::SmallVecBuf,
    x64::mem::addr::{Address, VirtAddr},
};

static AP_REMAINING: AtomicUsize = AtomicUsize::new(0);

/// # Safety
/// Needs to be called after all harts have started or failed to start
pub unsafe fn boot_kernel(kernel: &Elf<'static>, stacks: &mut SmallVecBuf<VirtAddr>) -> ! {
    let entry = kernel.entry;
    let entry = entry.as_usize();
    let kernel_init: KernelInitFn = unsafe {
        // SAFETY: sometimes rust sucks
        mem::transmute(entry)
    };

    let bsp_stack = stacks.pop().expect("Did not find stack for BSP");

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

    do_jump(bsp_stack.as_usize(), kernel_init as usize, true);
}

pub fn ap_boot_kernel(stack: usize, ap_entry: KernelInitFn) {
    AP_REMAINING.fetch_sub(1, Ordering::Relaxed);
    do_jump(stack, ap_entry as usize, false);
}

fn do_jump(stack: usize, dest: usize, is_bsp: bool) -> ! {
    unsafe {
        asm!(
            "mov rsp, {stack}",
            "jmp {entry}",
            stack = in(reg) stack,
            entry = in(reg) dest,
            in("rdi") is_bsp as u64,
            options(noreturn)
        );
    }
}
