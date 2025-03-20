#![no_std]
#![no_main]

mod panic;

use core::arch::asm;

#[unsafe(no_mangle)]
fn bsp_entry() {
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
