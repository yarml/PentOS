use core::arch::asm;

pub extern "C" fn bsp_entry() {
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

pub extern "C" fn ap_entry() {
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
