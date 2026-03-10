#![no_std]
#![no_main]

mod version;

use {
    klib::{
        bootinfo::bootinfo,
        dev::framebuffer,
        task::{self, sleep::sleep},
    },
    log::info,
    system::framebuffer::PixelColor,
    version::VERSION,
    x64::interrupts,
};

klib::use_klib!(kmain);

// Currently kmain is just testing the async system

async fn kmain() {
    info!("PentOS v{VERSION}");
    let hartcount = bootinfo().topology.harts.len();

    for i in 0..hartcount {
        task::spawn(test_task(i));
    }
    task::spawn(refresh_task());
}

async fn test_task(i: usize) {
    let (red, green, blue) = {
        let i = i + 1;
        let mut red = 0;
        let mut green = 0;
        let mut blue = 0;

        if i & 0x04 != 0 {
            red += 127;
        }
        if i & 0x20 != 0 {
            red += 127;
        }

        if i & 0x02 != 0 {
            green += 127;
        }
        if i & 0x10 != 0 {
            green += 127;
        }

        if i & 0x01 != 0 {
            blue += 127;
        }
        if i & 0x08 != 0 {
            blue += 127;
        }

        (red, green, blue)
    };

    let mut state = true;
    loop {
        interrupts::with_disabled(|| {
            let mut fb = framebuffer::lock();
            let color = if state {
                PixelColor(red, green, blue)
            } else {
                PixelColor(0, 0, 0)
            };
            fb.draw_box((i + 1) * 10, 10, 10, 10, color);
        });
        state = !state;
        sleep((i + 1) * 500).await
    }
}

async fn refresh_task() {
    loop {
        interrupts::with_disabled(|| {
            let mut fb = framebuffer::lock();
            fb.refresh();
        });
        sleep(20).await;
    }
}
