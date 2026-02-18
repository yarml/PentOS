#![no_std]
#![no_main]

extern crate alloc;

mod init;

use {alloc::boxed::Box, core::hint, log::debug};

/// # Safety
/// Should be called once in the BSP by klib
/// Assumes klib is fully functioning
unsafe fn kmain() -> ! {
    let a = Box::new(10);
    let b = Box::new(20);

    {
        let c = Box::new(30);
        debug!("a: {a}, b: {b}, c: {c}");
        debug!(
            "&a: {ra:?}, &b: {rb:?}, &c: {rc:?}",
            ra = a.as_ref() as *const i32,
            rb = b.as_ref() as *const i32,
            rc = c.as_ref() as *const i32,
        );
    }

    let c = Box::new(30);
    debug!("a: {a}, b: {b}, c: {c}");
    debug!(
        "&a: {ra:?}, &b: {rb:?}, &c: {rc:?}",
        ra = a.as_ref() as *const i32,
        rb = b.as_ref() as *const i32,
        rc = c.as_ref() as *const i32,
    );

    loop {
        hint::spin_loop();
    }
}
