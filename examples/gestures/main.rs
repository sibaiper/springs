//! Drag, flick, snap — the reason UI springs exist.
//!
//! Both panels here do the same thing at the moment you let go: measure how
//! fast your finger was moving, hand that number straight to the spring with
//! `set_velocity`, and then point the spring at wherever it should end up with
//! `set_target`. Because retargeting leaves the velocity alone, the throw and
//! the settle are one continuous motion rather than two animations stitched
//! together — there is no "now play the snap-back tween" step anywhere below.
//!
//! - **Carousel** — a paging strip. A slow release snaps to the nearest page; a
//!   flick carries to the next one, because the release velocity decides the
//!   target rather than the position alone.
//! - **Scroll** — overscroll and rubber band. Drag past the end and the list
//!   resists at 35%; let go and the same spring pulls it back, keeping whatever
//!   momentum the drag had.
//!
//! ```text
//! cargo run --example gestures --release
//! ```

#[path = "../common/canvas.rs"]
mod canvas;
#[path = "../common/font.rs"]
mod font;

use std::collections::VecDeque;
use std::time::Instant;

use canvas::Canvas;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use springs::{Spring, SpringConfig};

const WIDTH: i32 = 1100;
const HEIGHT: i32 = 780;
const MARGIN: i32 = 28;

const BACKGROUND: u32 = 0x0D1117;
const PANEL: u32 = 0x11161D;
const PANEL_EDGE: u32 = 0x222A35;
const RAIL: u32 = 0x1B222C;
const TEXT_DIM: u32 = 0x6E7B8B;
const TEXT: u32 = 0x9FB0C3;
const TEXT_BRIGHT: u32 = 0xE6EDF3;
const ACCENT: u32 = 0x79C0FF;
const GREEN: u32 = 0x7EE787;
const ORANGE: u32 = 0xFFA657;
const PURPLE: u32 = 0xD2A8FF;

// Carousel geometry.
const CARDS: usize = 6;
const CARD_WIDTH: f64 = 232.0;
const CARD_GAP: f64 = 26.0;
const PITCH: f64 = CARD_WIDTH + CARD_GAP;
const STRIP_TOP: f64 = 132.0;
const STRIP_HEIGHT: f64 = 214.0;

/// Release speed, in pixels per second, above which a drag counts as a flick.
const FLICK: f64 = 320.0;

// Scroll list geometry.
const LIST_LEFT: f64 = MARGIN as f64;
const LIST_WIDTH: f64 = 470.0;
const LIST_TOP: f64 = 434.0;
const LIST_HEIGHT: f64 = 286.0;
const ROW_HEIGHT: f64 = 42.0;
const ROWS: usize = 14;

/// How far past the end a drag actually moves, as a fraction of the overscroll.
const RESISTANCE: f64 = 0.35;

// ---------------------------------------------------------------------------
// Gesture velocity
// ---------------------------------------------------------------------------

/// Estimates release velocity the way a real gesture recogniser does: over a
/// short trailing window, so a stationary pause before letting go produces a
/// throw of zero rather than whatever the last frame happened to measure.
#[derive(Default)]
struct Velocity {
    samples: VecDeque<(f64, f64)>,
    clock: f64,
}

impl Velocity {
    const WINDOW: f64 = 0.09;

    fn restart(&mut self) {
        self.samples.clear();
        self.clock = 0.0;
    }

    fn push(&mut self, dt: f64, position: f64) {
        self.clock += dt;
        self.samples.push_back((self.clock, position));

        while let Some(&(when, _)) = self.samples.front() {
            if self.clock - when > Self::WINDOW {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    fn get(&self) -> f64 {
        match (self.samples.front(), self.samples.back()) {
            (Some(&(t0, p0)), Some(&(t1, p1))) if t1 - t0 > 1e-3 => (p1 - p0) / (t1 - t0),
            _ => 0.0,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Drag {
    None,
    Carousel,
    List,
}

struct Gestures {
    /// Scroll offset in pixels; page `n` sits at `n * PITCH`.
    carousel: Spring<f64>,
    list: Spring<f64>,

    drag: Drag,
    grab: (f64, f64),
    velocity: Velocity,

    last_flick: f64,
}

fn strip_left() -> f64 {
    f64::from(MARGIN) + 30.0
}

fn max_scroll() -> f64 {
    (ROWS as f64 * ROW_HEIGHT - LIST_HEIGHT).max(0.0)
}

impl Gestures {
    fn new() -> Self {
        Self {
            // Paging wants to land exactly on the page, so: no bounce.
            carousel: Spring::new(0.0)
                .with_config(SpringConfig::new().duration(0.55).bounce(0.0))
                .with_epsilon(0.4),
            list: Spring::new(0.0)
                .with_config(SpringConfig::new().duration(0.5).bounce(0.0))
                .with_epsilon(0.4),
            drag: Drag::None,
            grab: (0.0, 0.0),
            velocity: Velocity::default(),
            last_flick: 0.0,
        }
    }

    fn page(&self) -> usize {
        (self.carousel.value() / PITCH)
            .round()
            .clamp(0.0, (CARDS - 1) as f64) as usize
    }

    fn update(&mut self, mouse: (f64, f64), held: bool, clicked: bool, dt: f64) {
        if clicked {
            let in_strip = (STRIP_TOP..STRIP_TOP + STRIP_HEIGHT).contains(&mouse.1);
            let in_list = (LIST_TOP..LIST_TOP + LIST_HEIGHT).contains(&mouse.1)
                && (LIST_LEFT..LIST_LEFT + LIST_WIDTH).contains(&mouse.0);

            if in_strip {
                self.drag = Drag::Carousel;
                self.grab = (mouse.0, self.carousel.value());
            } else if in_list {
                self.drag = Drag::List;
                self.grab = (mouse.1, self.list.value());
            }

            self.velocity.restart();
        }

        match self.drag {
            Drag::Carousel => {
                // Dragging owns the value outright; the spring is only along
                // for the ride until the release.
                let offset = self.grab.1 - (mouse.0 - self.grab.0);
                let limit = (CARDS - 1) as f64 * PITCH;
                let offset = resist(offset, 0.0, limit);

                self.carousel.snap_to(offset);
                self.velocity.push(dt, offset);
            }
            Drag::List => {
                let offset = self.grab.1 - (mouse.1 - self.grab.0);
                let offset = resist(offset, 0.0, max_scroll());

                self.list.snap_to(offset);
                self.velocity.push(dt, offset);
            }
            Drag::None => {}
        }

        if !held && self.drag != Drag::None {
            let thrown = self.velocity.get();
            self.last_flick = thrown;

            match self.drag {
                Drag::Carousel => {
                    let page = self.carousel.value() / PITCH;

                    // The flick, not the position, chooses the page.
                    let landing = if thrown > FLICK {
                        page.floor() + 1.0
                    } else if thrown < -FLICK {
                        page.ceil() - 1.0
                    } else {
                        page.round()
                    };
                    let landing = landing.clamp(0.0, (CARDS - 1) as f64);

                    self.carousel.set_target(landing * PITCH);
                    self.carousel.set_velocity(thrown);
                }
                Drag::List => {
                    // Project where the throw is heading, then clamp it to the
                    // ends — which is what turns an overscroll into a snap back.
                    let projected = self.list.value() + thrown * 0.16;

                    self.list.set_target(projected.clamp(0.0, max_scroll()));
                    self.list.set_velocity(thrown);
                }
                Drag::None => {}
            }

            self.drag = Drag::None;
        }

        // Whichever panel is not under the finger keeps animating.
        if self.drag != Drag::Carousel {
            self.carousel.advance(dt);
        }
        if self.drag != Drag::List {
            self.list.advance(dt);
        }
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.text(MARGIN, 22, 2, TEXT_BRIGHT, "SPRINGS");
        canvas.text(
            MARGIN + font::width("SPRINGS", 2) + 16,
            26,
            1,
            TEXT_DIM,
            "GESTURES - DRAG, FLICK, SNAP",
        );
        canvas.rect(MARGIN, 46, WIDTH - 2 * MARGIN, 1, PANEL_EDGE);

        self.draw_carousel(canvas);
        self.draw_list(canvas);
        self.draw_readout(canvas);

        canvas.text(
            MARGIN,
            HEIGHT - 22,
            1,
            TEXT_DIM,
            "DRAG EITHER PANEL AND LET GO   R: RESET   ESC: QUIT",
        );
    }

    fn draw_carousel(&self, canvas: &mut Canvas) {
        canvas.text(MARGIN, 74, 2, TEXT_BRIGHT, "CAROUSEL");
        canvas.text(
            MARGIN,
            94,
            1,
            TEXT_DIM,
            "FLICK PAST 320 PX/S TO TURN THE PAGE, OR RELEASE SLOWLY TO SNAP BACK",
        );

        let offset = self.carousel.value();

        for index in 0..CARDS {
            let x = strip_left() + index as f64 * PITCH - offset;
            if x + CARD_WIDTH < 0.0 || x > f64::from(WIDTH) {
                continue;
            }

            let current = index == self.page();
            let colour = [ACCENT, GREEN, ORANGE, PURPLE, ACCENT, GREEN][index % 6];
            let body = canvas::mix(PANEL, colour, if current { 0.22 } else { 0.07 });

            canvas.rounded_rect(x, STRIP_TOP, CARD_WIDTH, STRIP_HEIGHT, 14.0, body);
            canvas.rounded_rect(x, STRIP_TOP, CARD_WIDTH, 4.0, 2.0, colour);

            canvas.text(
                (x + 22.0) as i32,
                (STRIP_TOP + 34.0) as i32,
                4,
                if current { TEXT_BRIGHT } else { TEXT_DIM },
                &format!("{}", index + 1),
            );
            canvas.text(
                (x + 22.0) as i32,
                (STRIP_TOP + 92.0) as i32,
                1,
                TEXT_DIM,
                &format!("PAGE AT {:.0} PX", index as f64 * PITCH),
            );
        }

        // The cards float on the background rather than inside a panel, so
        // masking the two margins is the whole of the clipping.
        canvas.rect(
            0,
            STRIP_TOP as i32 - 2,
            MARGIN,
            STRIP_HEIGHT as i32 + 4,
            BACKGROUND,
        );
        canvas.rect(
            WIDTH - MARGIN,
            STRIP_TOP as i32 - 2,
            MARGIN,
            STRIP_HEIGHT as i32 + 4,
            BACKGROUND,
        );

        // Page dots.
        for index in 0..CARDS {
            let x = f64::from(WIDTH) / 2.0 + (index as f64 - (CARDS - 1) as f64 / 2.0) * 22.0;
            let here = index == self.page();

            canvas.disc(
                x,
                STRIP_TOP + STRIP_HEIGHT + 26.0,
                if here { 5.0 } else { 3.0 },
                if here { TEXT_BRIGHT } else { RAIL },
            );
        }
    }

    fn draw_list(&self, canvas: &mut Canvas) {
        canvas.text(MARGIN, LIST_TOP as i32 - 60, 2, TEXT_BRIGHT, "SCROLL");
        canvas.text(
            MARGIN,
            LIST_TOP as i32 - 40,
            1,
            TEXT_DIM,
            "DRAG PAST EITHER END TO FEEL THE RUBBER BAND",
        );

        canvas.rect(
            LIST_LEFT as i32,
            LIST_TOP as i32,
            LIST_WIDTH as i32,
            LIST_HEIGHT as i32,
            PANEL,
        );

        let offset = self.list.value();
        for index in 0..ROWS {
            let y = LIST_TOP + index as f64 * ROW_HEIGHT - offset;
            if y + ROW_HEIGHT < LIST_TOP - ROW_HEIGHT || y > LIST_TOP + LIST_HEIGHT + ROW_HEIGHT {
                continue;
            }

            canvas.rect(
                LIST_LEFT as i32 + 1,
                (y + ROW_HEIGHT - 1.0) as i32,
                LIST_WIDTH as i32 - 2,
                1,
                RAIL,
            );
            canvas.disc(LIST_LEFT + 26.0, y + ROW_HEIGHT / 2.0, 7.0, RAIL);
            canvas.text(
                LIST_LEFT as i32 + 46,
                (y + ROW_HEIGHT / 2.0 - 3.0) as i32,
                1,
                TEXT,
                &format!("ITEM {:02}", index + 1),
            );
        }

        // Mask above and below, then redraw the frame.
        canvas.rect(
            LIST_LEFT as i32,
            LIST_TOP as i32 - 60,
            LIST_WIDTH as i32,
            60,
            BACKGROUND,
        );
        canvas.rect(
            LIST_LEFT as i32,
            (LIST_TOP + LIST_HEIGHT) as i32,
            LIST_WIDTH as i32,
            60,
            BACKGROUND,
        );
        canvas.outline(
            LIST_LEFT as i32,
            LIST_TOP as i32,
            LIST_WIDTH as i32,
            LIST_HEIGHT as i32,
            PANEL_EDGE,
        );

        // Overscroll indicator: how far past the end the list currently is.
        let over = offset - offset.clamp(0.0, max_scroll());
        if over.abs() > 0.5 {
            let y = if over < 0.0 {
                LIST_TOP + 4.0
            } else {
                LIST_TOP + LIST_HEIGHT - 8.0
            };
            canvas.rounded_rect(
                LIST_LEFT + 8.0,
                y,
                (LIST_WIDTH - 16.0) * (over.abs() / 90.0).min(1.0),
                4.0,
                2.0,
                ORANGE,
            );
        }
    }

    fn draw_readout(&self, canvas: &mut Canvas) {
        let x = LIST_LEFT + LIST_WIDTH + 44.0;
        let width = f64::from(WIDTH - MARGIN) - x;

        canvas.rect(
            x as i32,
            LIST_TOP as i32,
            width as i32,
            LIST_HEIGHT as i32,
            PANEL,
        );
        canvas.outline(
            x as i32,
            LIST_TOP as i32,
            width as i32,
            LIST_HEIGHT as i32,
            PANEL_EDGE,
        );

        let text_x = x as i32 + 20;
        canvas.text(text_x, LIST_TOP as i32 + 18, 2, TEXT_BRIGHT, "LIVE");

        let rows = [
            format!("CAROUSEL OFFSET   {:>8.1} PX", self.carousel.value()),
            format!("CAROUSEL VELOCITY {:>8.1} PX/S", self.carousel.velocity()),
            format!("PAGE              {:>8}", self.page() + 1),
            String::new(),
            format!("SCROLL OFFSET     {:>8.1} PX", self.list.value()),
            format!("SCROLL VELOCITY   {:>8.1} PX/S", self.list.velocity()),
            format!("SCROLL RANGE      0 TO {:.0} PX", max_scroll()),
            String::new(),
            format!("LAST RELEASE      {:>8.1} PX/S", self.last_flick),
            format!(
                "COUNTED AS        {}",
                if self.last_flick.abs() > FLICK {
                    "A FLICK"
                } else {
                    "A SLOW RELEASE"
                }
            ),
        ];

        for (index, row) in rows.iter().enumerate() {
            canvas.text(
                text_x,
                LIST_TOP as i32 + 52 + index as i32 * 16,
                1,
                TEXT,
                row,
            );
        }

        canvas.text(
            text_x,
            (LIST_TOP + LIST_HEIGHT) as i32 - 42,
            1,
            TEXT_DIM,
            "ON RELEASE: SET_VELOCITY THEN SET_TARGET.",
        );
        canvas.text(
            text_x,
            (LIST_TOP + LIST_HEIGHT) as i32 - 28,
            1,
            TEXT_DIM,
            "THE ORDER DOES NOT MATTER - NEITHER",
        );
        canvas.text(
            text_x,
            (LIST_TOP + LIST_HEIGHT) as i32 - 14,
            1,
            TEXT_DIM,
            "ONE DISTURBS THE OTHER.",
        );
    }
}

/// Past either end, a drag only moves by a fraction of the distance.
fn resist(offset: f64, low: f64, high: f64) -> f64 {
    let clamped = offset.clamp(low, high);

    clamped + (offset - clamped) * RESISTANCE
}

fn main() -> Result<(), minifb::Error> {
    let mut window = Window::new(
        "springs - gestures",
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions::default(),
    )?;
    window.set_target_fps(120);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut gestures = Gestures::new();
    let mut was_held = false;
    let mut clock = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = clock.elapsed().as_secs_f64().min(0.1);
        clock = Instant::now();

        let mouse = window
            // Clamped, so a drag that wanders outside the window keeps
            // tracking the edge instead of teleporting to the origin.
            .get_mouse_pos(MouseMode::Clamp)
            .map(|(x, y)| (f64::from(x), f64::from(y)))
            .unwrap_or((0.0, 0.0));
        let held = window.get_mouse_down(MouseButton::Left);

        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            gestures = Gestures::new();
        }

        gestures.update(mouse, held, held && !was_held, dt);
        was_held = held;

        canvas.clear(BACKGROUND);
        gestures.draw(&mut canvas);

        window.update_with_buffer(canvas.buffer(), WIDTH as usize, HEIGHT as usize)?;
    }

    Ok(())
}
