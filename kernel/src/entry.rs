use boot_protocol::kernel_meta::KernelMeta;
use core::arch::asm;
use x64::mem::addr::Address;
use x64::mem::addr::VirtAddr;

#[unsafe(no_mangle)]
extern "C" fn kernel_meta() -> KernelMeta {
    KernelMeta {
        bsp_entry: VirtAddr::new_panic(bsp_entry as usize),
        ap_entry: VirtAddr::new_panic(ap_entry as usize),
    }
}

extern "C" fn bsp_entry() {
    loop {
        unsafe {
            asm!(
                "hlt",
                // Just to see this in the debugger and know we're in the kernel, not the bootloader
                "xor eax, eax",
                "xor eax, eax",
                "xor eax, eax",
                "xor eax, eax",
            );
        }
    }
}

extern "C" fn ap_entry() {
    loop {
        unsafe {
            asm!(
                "hlt",
                "xor ecx, ecx",
                "xor ecx, ecx",
                "xor ecx, ecx",
                "xor ecx, ecx",
            );
        }
    }
}
