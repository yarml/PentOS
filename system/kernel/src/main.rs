#![no_std]
#![no_main]

mod version;

use {
    console_font::{FontStateMachine, GLYPH_WIDTH, GLYPHS, GlyphResult},
    klib::{
        dev::{
            framebuffer::{self, Framebuffer},
            ps2::{KeyEventStream, key_event_stream},
            timer::get_timestamp,
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
    task::spawn(timestamp_task());
    task::spawn(refresh_task());
    for i in 0..20 {
        task::spawn(test_task(i));
    }

    {
        let mut fb = framebuffer::lock().await;
        draw_str(
            &mut fb,
            "Hello World! :) <> ^        UTF-8 works???",
            10,
            50,
            PixelColor(255, 255, 255),
            PixelColor(0, 0, 0),
        );
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
    let mut stream = key_event_stream().await;
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
    let mut stream = key_event_stream().await;
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

async fn timestamp_task() {
    loop {
        let time = get_timestamp();
        {
            let mut fb = framebuffer::lock().await;
            for i in 0..64 {
                let bit = (time >> i) & 1 == 1;
                let color = if bit {
                    PixelColor(0, 0, 255)
                } else {
                    PixelColor(255, 0, 0)
                };

                fb.draw_box((64 - i) * 20, 30, 10, 10, color);
            }
        }
        sleep(10).await
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

pub fn draw_str(fb: &mut Framebuffer, s: &str, x: usize, y: usize, fg: PixelColor, bg: PixelColor) {
    let mut machine = FontStateMachine::new();
    let mut cursor_x = x;

    for byte in s.bytes() {
        match machine.feed(byte) {
            GlyphResult::Incomplete => {}
            GlyphResult::Found(glyph_index) | GlyphResult::Fallback(glyph_index) => {
                draw_glyph(fb, glyph_index, cursor_x, y, fg, bg);
                cursor_x += GLYPH_WIDTH;
            }
        }
    }
}

fn draw_glyph(
    fb: &mut Framebuffer,
    glyph_index: u16,
    x: usize,
    y: usize,
    fg: PixelColor,
    bg: PixelColor,
) {
    let glyph = &GLYPHS[glyph_index as usize];
    for (row, glyph_line) in glyph.iter().enumerate() {
        for col in 0..GLYPH_WIDTH {
            let byte_index = col / 8;
            let bit_index = 7 - (col % 8); // MSB first
            let set = (glyph_line[byte_index] >> bit_index) & 1 != 0;
            fb.set_pixel(x + col, y + row, if set { fg } else { bg });
        }
    }
}
