use core::arch::asm;

pub extern "sysv64" fn bsp_entry() {
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

pub extern "sysv64" fn ap_entry() {
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
