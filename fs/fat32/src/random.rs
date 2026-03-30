#[cfg(target_os = "none")]
pub fn gen_u32() -> u32 {
    use core::arch::asm;

    let val: u32;
    loop {
        let success: u8;
        unsafe {
            asm!(
                "rdrand {val:e}",
                "setc {success}",
                val = out(reg) val,
                success = out(reg_byte) success,
            );
        }
        if success != 0 {
            break;
        }
    }

    val
}

#[cfg(not(target_os = "none"))]
pub fn gen_u32() -> u32 {
    use getrandom;
    (getrandom::u64().expect("gerandom failed") & 0xFFFFFFFF) as u32
}
