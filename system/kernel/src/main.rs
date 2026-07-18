#![no_std]
#![no_main]

mod version;

use {
    crate::version::VERSION, alloc::format, console_font::GLYPH_HEIGHT, log::info,
    system::framebuffer::PixelColor,
};

klib::use_klib!(kmain);

async fn kmain() {
    info!("PentOS v{VERSION}");

    let mut fb = framebuffer::lock().await;

    for (line_n, (func_addr, info)) in pci::walk().enumerate() {
        let line = format!("{func_addr}: {info}");
        fb.draw_str(
            &line,
            10,
            line_n * GLYPH_HEIGHT,
            PixelColor::WHITE,
            PixelColor::BLACK,
        );
        fb.refresh();
        ps2::keyboard_update().await.await;
    }

    loop {
        fb.draw_box(500, 50, 10, 10, PixelColor::RED);
        fb.refresh();
        timer::sleep::sleep(1000).await;
        fb.draw_box(500, 50, 10, 10, PixelColor::BLUE);
        fb.refresh();
        timer::sleep::sleep(1000).await;
    }
}
