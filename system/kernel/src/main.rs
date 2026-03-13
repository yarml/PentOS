#![no_std]
#![no_main]

mod version;

use {
    alloc::vec,
    klib::{
        bootinfo::bootinfo,
        dev::{
            framebuffer,
            ps2::{
                self,
                keys::{KEY_CURSOR_DOWN, KEY_CURSOR_LEFT, KEY_CURSOR_RIGHT, KEY_CURSOR_UP},
            },
        },
        task::{self, sleep::sleep},
    },
    log::info,
    system::framebuffer::PixelColor,
    version::VERSION,
};

klib::use_klib!(kmain);

async fn kmain() {
    info!("PentOS v{VERSION} - special snake");

    let bootinfo = bootinfo();
    let resolution = (bootinfo.framebuffer.width, bootinfo.framebuffer.height);

    task::spawn(game(resolution));
    task::spawn(refresh());
}

async fn game(resolution: (usize, usize)) {
    const GRID_DIMENSIONS: (isize, isize) = (64, 48);

    let cell_size = (
        resolution.0 / GRID_DIMENSIONS.0 as usize,
        resolution.1 / GRID_DIMENSIONS.1 as usize,
    );

    fn random_coords() -> (isize, isize) {
        let x_base = get_rand() % GRID_DIMENSIONS.0 as u64;
        let y_base = get_rand() % GRID_DIMENSIONS.1 as u64;

        let x = x_base as isize - GRID_DIMENSIONS.0 / 2;
        let y = y_base as isize - GRID_DIMENSIONS.1 / 2;
        (x, y)
    }

    fn grid_to_pixel(
        cell: (isize, isize),
        cell_size: (usize, usize),
        grid_dim: (isize, isize),
    ) -> (usize, usize) {
        let px = (cell.0 + grid_dim.0 / 2) as usize * cell_size.0;
        let py = (cell.1 + grid_dim.1 / 2) as usize * cell_size.1;
        (px, py)
    }

    let mut snake = vec![(0isize, 0isize), (-1, 0), (-2, 0), (-3, 0)];
    let mut direction = Direction::Right;

    let mut apple = loop {
        let p = random_coords();
        if !snake.contains(&p) {
            break p;
        }
    };

    const COLOR_SNAKE_HEAD: PixelColor = PixelColor(0, 255, 80);
    const COLOR_SNAKE_BODY: PixelColor = PixelColor(0, 180, 50);
    const COLOR_APPLE: PixelColor = PixelColor(255, 50, 50);

    let mut game_over = false;

    loop {
        if direction.is_horizontal() {
            if ps2::is_down(KEY_CURSOR_UP) {
                direction = Direction::Up;
            } else if ps2::is_down(KEY_CURSOR_DOWN) {
                direction = Direction::Down;
            }
        } else if direction.is_vertical() {
            if ps2::is_down(KEY_CURSOR_LEFT) {
                direction = Direction::Left;
            } else if ps2::is_down(KEY_CURSOR_RIGHT) {
                direction = Direction::Right;
            }
        }

        if !game_over {
            // Compute new head position
            let (dx, dy) = direction.relative_coords();
            let head = snake[0];
            let new_head = (head.0 + dx, head.1 + dy);

            // Check wall collision
            let hit_wall = new_head.0 < -GRID_DIMENSIONS.0 / 2
                || new_head.0 >= GRID_DIMENSIONS.0 / 2
                || new_head.1 < -GRID_DIMENSIONS.1 / 2
                || new_head.1 >= GRID_DIMENSIONS.1 / 2;

            // Check self collision (ignore tail tip — it will move away)
            let hit_self = snake[..snake.len() - 1].contains(&new_head);

            if hit_wall || hit_self {
                game_over = true;
            } else {
                let ate_apple = new_head == apple;

                // Move snake: prepend new head
                snake.insert(0, new_head);

                if ate_apple {
                    // Don't remove tail — snake grows
                    // Spawn new apple
                    apple = loop {
                        let p = random_coords();
                        if !snake.contains(&p) {
                            break p;
                        }
                    };
                } else {
                    // Remove tail
                    snake.pop();
                }
            }
        }

        // Draw frame
        {
            let mut fb = framebuffer::lock().await;

            // Clear screen
            fb.clear();

            if game_over {
                // Draw a red banner across the middle to indicate game over
                let banner_y = resolution.1 / 2 - cell_size.1;
                fb.draw_box(
                    0,
                    banner_y,
                    resolution.0,
                    cell_size.1 * 2,
                    PixelColor(180, 0, 0),
                );
            } else {
                // Draw apple
                let (ax, ay) = grid_to_pixel(apple, cell_size, GRID_DIMENSIONS);
                fb.draw_box(
                    ax + 1,
                    ay + 1,
                    cell_size.0.saturating_sub(2),
                    cell_size.1.saturating_sub(2),
                    COLOR_APPLE,
                );

                // Draw snake
                for (i, &cell) in snake.iter().enumerate() {
                    let (px, py) = grid_to_pixel(cell, cell_size, GRID_DIMENSIONS);
                    let color = if i == 0 {
                        COLOR_SNAKE_HEAD
                    } else {
                        COLOR_SNAKE_BODY
                    };
                    fb.draw_box(
                        px + 1,
                        py + 1,
                        cell_size.0.saturating_sub(2),
                        cell_size.1.saturating_sub(2),
                        color,
                    );
                }
            }
        }

        sleep(150).await;
    }
}

async fn refresh() {
    loop {
        {
            let mut fb = framebuffer::lock().await;
            fb.refresh();
        }
        sleep(40).await; // Target 25 FPS
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    const fn is_vertical(&self) -> bool {
        matches!(*self, Direction::Up | Direction::Down)
    }
    const fn is_horizontal(&self) -> bool {
        !self.is_vertical()
    }

    const fn relative_coords(&self) -> (isize, isize) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

fn get_rand() -> u64 {
    let mut val: u64 = 0;

    unsafe {
        core::arch::asm!(
            "rdrand {val}",
            val = out(reg) val,
        );
    }

    val
}
