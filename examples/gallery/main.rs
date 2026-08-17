//! A gallery of spring demos, one per tab.
//!
//! Each scene showcases a different part of the crate:
//!
//! - **Follow** — a chain of `Spring<[f64; 2]>` nodes chasing the mouse, with
//!   the config tunable live. Two-dimensional springs, and what duration and
//!   bounce actually feel like.
//! - **Compass** — a `Spring<Angle>` against a naive `Spring<f64>` on raw
//!   degrees, side by side. The wrapped one takes the short way round; the
//!   naive one sails the long way, and a travel counter shows the difference.
//! - **Interface** — the things springs are actually for: a sliding tab pill,
//!   a toggle, a sheet, and a staggered list.
//! - **Phase** — displacement against velocity for all three damping regimes
//!   at once, so underdamped spirals and overdamped goes straight in.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example gallery --release
//! ```

#[path = "../common/canvas.rs"]
mod canvas;
#[path = "../common/font.rs"]
mod font;

mod compass;
mod follow;
mod interface;
mod phase;

use std::time::Instant;

use canvas::Canvas;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

// ---------------------------------------------------------------------------
// Shared layout and palette
// ---------------------------------------------------------------------------

pub const WIDTH: i32 = 1100;
pub const HEIGHT: i32 = 780;
pub const MARGIN: i32 = 28;

pub const CONTENT_TOP: i32 = 78;
pub const CONTENT_BOTTOM: i32 = HEIGHT - 42;

pub const BACKGROUND: u32 = 0x0D1117;
pub const PANEL: u32 = 0x11161D;
pub const PANEL_EDGE: u32 = 0x222A35;
pub const RAIL: u32 = 0x1B222C;
pub const TEXT_DIM: u32 = 0x6E7B8B;
pub const TEXT: u32 = 0x9FB0C3;
pub const TEXT_BRIGHT: u32 = 0xE6EDF3;

pub const ACCENT: u32 = 0x79C0FF;
pub const GREEN: u32 = 0x7EE787;
pub const ORANGE: u32 = 0xFFA657;
pub const PURPLE: u32 = 0xD2A8FF;
pub const RED: u32 = 0xFF7B72;

/// Everything a scene needs to know about this frame.
pub struct Input<'a> {
    pub window: &'a Window,
    pub mouse: (f64, f64),
    pub clicked: bool,
    pub held: bool,
    pub dt: f64,
}

impl Input<'_> {
    pub fn pressed(&self, key: Key) -> bool {
        self.window.is_key_pressed(key, KeyRepeat::No)
    }

    pub fn down(&self, key: Key) -> bool {
        self.window.is_key_down(key)
    }
}

/// Draws a caption and, under it, the config that produced what is below.
pub fn caption(canvas: &mut Canvas, x: i32, y: i32, title: &str, detail: &str) {
    canvas.text(x, y, 2, TEXT_BRIGHT, title);
    canvas.text(x, y + 20, 1, TEXT_DIM, detail);
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

const TABS: [&str; 4] = ["FOLLOW", "COMPASS", "INTERFACE", "PHASE"];

fn main() -> Result<(), minifb::Error> {
    let mut window = Window::new(
        "springs - gallery",
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions::default(),
    )?;
    window.set_target_fps(120);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);

    let mut follow = follow::Follow::new();
    let mut compass = compass::Compass::new();
    let mut interface = interface::Interface::new();
    let mut phase = phase::Phase::new();

    let mut active = 0usize;
    let mut was_held = false;
    let mut clock = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = clock.elapsed().as_secs_f64().min(0.1);
        clock = Instant::now();

        let mouse = window
            .get_mouse_pos(MouseMode::Clamp)
            .map(|(x, y)| (f64::from(x), f64::from(y)))
            .unwrap_or((0.0, 0.0));

        let held = window.get_mouse_down(MouseButton::Left);
        let input = Input {
            window: &window,
            mouse,
            clicked: held && !was_held,
            held,
            dt,
        };
        was_held = held;

        for (index, key) in [Key::Key1, Key::Key2, Key::Key3, Key::Key4]
            .into_iter()
            .enumerate()
        {
            if window.is_key_pressed(key, KeyRepeat::No) {
                active = index;
            }
        }
        if window.is_key_pressed(Key::Tab, KeyRepeat::No) {
            active = (active + 1) % TABS.len();
        }

        canvas.clear(BACKGROUND);

        let help = match active {
            0 => {
                follow.update(&input);
                follow.draw(&mut canvas);
                follow::Follow::HELP
            }
            1 => {
                compass.update(&input);
                compass.draw(&mut canvas);
                compass::Compass::HELP
            }
            2 => {
                interface.update(&input);
                interface.draw(&mut canvas);
                interface::Interface::HELP
            }
            _ => {
                phase.update(&input);
                phase.draw(&mut canvas);
                phase::Phase::HELP
            }
        };

        draw_chrome(&mut canvas, active, help);

        window.update_with_buffer(canvas.buffer(), WIDTH as usize, HEIGHT as usize)?;
    }

    Ok(())
}

fn draw_chrome(canvas: &mut Canvas, active: usize, help: &str) {
    canvas.text(MARGIN, 22, 2, TEXT_BRIGHT, "SPRINGS");
    canvas.text(
        MARGIN + font::width("SPRINGS", 2) + 16,
        26,
        1,
        TEXT_DIM,
        "GALLERY",
    );

    // Tabs, right aligned.
    let mut x = WIDTH - MARGIN;
    for (index, tab) in TABS.iter().enumerate().rev() {
        let label = format!("{}. {tab}", index + 1);
        let width = font::width(&label, 1);
        x -= width;

        let selected = index == active;
        let ink = if selected { TEXT_BRIGHT } else { TEXT_DIM };
        canvas.text(x, 26, 1, ink, &label);

        if selected {
            canvas.rect(x, 38, width, 2, ACCENT);
        }

        x -= 26;
    }

    canvas.rect(MARGIN, 58, WIDTH - 2 * MARGIN, 1, PANEL_EDGE);
    canvas.rect(
        MARGIN,
        CONTENT_BOTTOM + 12,
        WIDTH - 2 * MARGIN,
        1,
        PANEL_EDGE,
    );

    canvas.text(MARGIN, HEIGHT - 22, 1, TEXT_DIM, help);
    let quit = "TAB: NEXT SCENE   ESC: QUIT";
    canvas.text(
        WIDTH - MARGIN - font::width(quit, 1),
        HEIGHT - 22,
        1,
        TEXT_DIM,
        quit,
    );
}
