#![no_std]
#![no_main]

mod version;

use {
    klib::{
        dev::{
            framebuffer,
            ps2::{KeyEventStream, key_event_stream, keyboard_update},
        },
        task::{self, sleep::sleep},
    },
    log::info,
    system::framebuffer::PixelColor,
    task::stream::Stream,
    version::VERSION,
};

klib::use_klib!(kmain);

// Currently kmain is just testing the async system

async fn kmain() {
    info!("PentOS v{VERSION}");

    task::spawn(kbd_task());
    task::spawn(kbd2_task());
    task::spawn(refresh_task());
    for i in 0..20 {
        task::spawn(test_task(i));
    }
}

async fn test_task(i: usize) {
    let color = get_color(i);
    let mut state = true;
    loop {
        {
            let mut fb = framebuffer::lock().await;
            let color = if state { color } else { PixelColor(0, 0, 0) };
            fb.draw_box((i + 2) * 10, 10, 10, 10, color);
        }
        state = !state;
        sleep((i + 1) * 50).await
    }
}

async fn refresh_task() {
    loop {
        {
            let mut fb = framebuffer::lock().await;
            fb.refresh();
        }
        sleep(20).await; // Target 50FPS
    }
}

async fn kbd_task() {
    let mut state = true;
    let mut stream = key_event_stream();
    loop {
        {
            let mut fb = framebuffer::lock().await;
            let color = if state {
                PixelColor(255, 255, 0)
            } else {
                PixelColor(0, 0, 0)
            };
            fb.draw_box(10, 20, 10, 10, color);
        }
        let kv = KeyEventStream::next(&mut stream)
            .await
            .expect("KeyboardEventStream does not finish");
        if kv.is_released() {
            state = !state;
        }
    }
}

async fn kbd2_task() {
    let mut state = true;
    let mut stream = key_event_stream();
    loop {
        {
            let mut fb = framebuffer::lock().await;
            let color = if state {
                PixelColor(255, 255, 0)
            } else {
                PixelColor(0, 0, 0)
            };
            fb.draw_box(21 * 10, 20, 10, 10, color);
        }
        let kv = KeyEventStream::next(&mut stream)
            .await
            .expect("KeyboardEventStream does not finish");
        if kv.is_pressed() {
            state = !state;
        }
    }
}

fn get_color(i: usize) -> PixelColor {
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

    PixelColor(red, green, blue)
}
