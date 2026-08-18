//! A visual test for the spring solver.
//!
//! Four springs with different damping share one target. Click anywhere on a
//! track to retarget them and watch the difference between the regimes: the
//! critical spring arrives without ever crossing the line, the bouncy ones
//! cross it repeatedly, the overdamped one crawls in behind everybody.
//!
//! The hollow ring chasing each puck is the frame-rate independence check.
//! The filled puck is stepped once per rendered frame with whatever `dt` the
//! display happened to produce; the ring is computed by advancing a snapshot
//! taken at the last retarget by the *total* elapsed time in a single call.
//! Because the solver is analytical the two are the same motion, so the ring
//! must stay welded to the puck — including in slow motion, where the frame
//! `dt` and the single jump differ by seconds. The header reports the worst
//! disagreement seen so far, which should stay at float noise.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example visual
//! ```

#[path = "../common/canvas.rs"]
mod canvas;
#[path = "../common/font.rs"]
mod font;

use std::collections::VecDeque;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use canvas::Canvas;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use springs::{Spring, SpringConfig};

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

const WIDTH: i32 = 1040;
const HEIGHT: i32 = 760;

const MARGIN: i32 = 24;
const TRACK_LEFT: i32 = 208;
const TRACK_RIGHT: i32 = WIDTH - MARGIN;

const PUCK_RADIUS: f64 = 11.0;
/// The springs animate in screen space: their value *is* a pixel column.
const TRACK_MIN: f64 = TRACK_LEFT as f64 + PUCK_RADIUS + 2.0;
const TRACK_MAX: f64 = TRACK_RIGHT as f64 - PUCK_RADIUS - 2.0;

const LANES_TOP: i32 = 76;
const LANE_HEIGHT: i32 = 74;
const LANE_COUNT: i32 = 4;

const TRACE_TOP: i32 = LANES_TOP + LANE_COUNT * LANE_HEIGHT + 20;
const TRACE_BOTTOM: i32 = HEIGHT - 44;
const TRACE_PADDING: f64 = 26.0;
const TRACE_SAMPLES: usize = (TRACK_RIGHT - TRACK_LEFT) as usize;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

const BACKGROUND: u32 = 0x0D1117;
const PANEL: u32 = 0x11161D;
const GRID: u32 = 0x222A35;
const RAIL: u32 = 0x1B222C;
const TEXT_DIM: u32 = 0x6E7B8B;
const TEXT: u32 = 0x9FB0C3;
const TEXT_BRIGHT: u32 = 0xE6EDF3;
const TARGET_INK: u32 = 0x4B6584;
const GHOST_INK: u32 = 0xE6EDF3;

// ---------------------------------------------------------------------------
// Lanes
// ---------------------------------------------------------------------------

struct Lane {
    label: &'static str,
    color: u32,

    /// Stepped once per rendered frame.
    spring: Spring<f64>,
    /// The state when the target was last moved, plus the time since.
    snapshot: Spring<f64>,
    elapsed: f64,

    trace: VecDeque<f64>,
}

impl Lane {
    fn new(label: &'static str, color: u32, config: SpringConfig) -> Self {
        let spring = Spring::new(TRACK_MIN).with_config(config);

        Self {
            label,
            color,

            spring,
            snapshot: spring,
            elapsed: 0.0,
            trace: VecDeque::with_capacity(TRACE_SAMPLES),
        }
    }

    fn retarget(&mut self, target: f64) {
        self.spring.set_target(target);
        self.restart_reference();
    }

    fn kick(&mut self, velocity: f64) {
        self.spring.add_velocity(velocity);
        self.restart_reference();
    }

    fn reset(&mut self) {
        self.spring.snap_to(TRACK_MIN);
        self.trace.clear();
        self.restart_reference();
    }

    /// Re-anchors the single-jump reference on the current state.
    fn restart_reference(&mut self) {
        self.snapshot = self.spring;
        self.elapsed = 0.0;
    }

    fn advance(&mut self, dt: f64) {
        self.spring.advance(dt);
        self.elapsed += dt;

        if self.trace.len() == TRACE_SAMPLES {
            self.trace.pop_front();
        }
        self.trace.push_back(self.spring.value());
    }

    /// The same interval, integrated in one analytical step instead of many.
    fn reference(&self) -> Spring<f64> {
        let mut reference = self.snapshot;
        reference.advance(self.elapsed);
        reference
    }

    fn center_y(index: usize) -> i32 {
        LANES_TOP + index as i32 * LANE_HEIGHT + LANE_HEIGHT / 2
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

fn main() -> Result<(), minifb::Error> {
    let mut window = Window::new(
        "springs - visual test",
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions::default(),
    )?;
    window.set_target_fps(120);

    let mut lanes = [
        Lane::new(
            "CRITICAL",
            0x7EE787,
            SpringConfig::new().duration(0.5).bounce(0.0),
        ),
        Lane::new(
            "GENTLE",
            0x79C0FF,
            SpringConfig::new().duration(0.5).bounce(0.3),
        ),
        Lane::new(
            "BOUNCY",
            0xFFA657,
            SpringConfig::new().duration(0.5).bounce(0.7),
        ),
        Lane::new(
            "OVERDAMPED",
            0xD2A8FF,
            SpringConfig::from_response_damping(0.5, 1.8),
        ),
    ];

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut random = Random::seeded();

    let mut target = TRACK_MIN;
    let mut target_trace: VecDeque<f64> = VecDeque::with_capacity(TRACE_SAMPLES);
    let mut speed = 1.0;
    let mut worst_drift = 0.0f64;
    let mut clock = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // A real frame `dt`, capped so that dragging the window does not
        // teleport every spring at once.
        let frame = clock.elapsed().as_secs_f64().min(0.1);
        clock = Instant::now();

        // --- input ---------------------------------------------------------

        if window.get_mouse_down(MouseButton::Left)
            && let Some((x, _)) = window.get_mouse_pos(MouseMode::Clamp)
            && f64::from(x) >= f64::from(TRACK_LEFT)
        {
            target = f64::from(x).clamp(TRACK_MIN, TRACK_MAX);
            for lane in &mut lanes {
                lane.retarget(target);
            }
        }

        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            target = TRACK_MIN + random.unit() * (TRACK_MAX - TRACK_MIN);
            for lane in &mut lanes {
                lane.retarget(target);
            }
        }

        if window.is_key_pressed(Key::V, KeyRepeat::No) {
            let velocity = if random.unit() < 0.5 { -1400.0 } else { 1400.0 };
            for lane in &mut lanes {
                lane.kick(velocity);
            }
        }

        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            target = TRACK_MIN;
            worst_drift = 0.0;
            target_trace.clear();
            for lane in &mut lanes {
                lane.reset();
            }
        }

        for (key, factor) in [(Key::Key1, 1.0), (Key::Key2, 0.25), (Key::Key3, 0.1)] {
            if window.is_key_pressed(key, KeyRepeat::No) {
                speed = factor;
            }
        }

        // --- simulate ------------------------------------------------------

        for lane in &mut lanes {
            lane.advance(frame * speed);
            worst_drift = worst_drift.max((lane.spring.value() - lane.reference().value()).abs());
        }

        if target_trace.len() == TRACE_SAMPLES {
            target_trace.pop_front();
        }
        target_trace.push_back(target);

        // --- draw ----------------------------------------------------------

        canvas.clear(BACKGROUND);
        draw_header(&mut canvas, worst_drift, speed);
        draw_lanes(&mut canvas, &lanes, target);
        draw_trace(&mut canvas, &lanes, &target_trace);
        draw_footer(&mut canvas);

        window.update_with_buffer(canvas.buffer(), WIDTH as usize, HEIGHT as usize)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn draw_header(canvas: &mut Canvas, worst_drift: f64, speed: f64) {
    canvas.text(MARGIN, 20, 2, TEXT_BRIGHT, "SPRINGS");
    canvas.text(
        MARGIN + font::width("SPRINGS", 2) + 16,
        24,
        1,
        TEXT_DIM,
        "ANALYTICAL SOLVER, VISUAL TEST",
    );

    canvas.text(
        MARGIN,
        46,
        1,
        TEXT,
        "PUCK STEPPED EVERY FRAME    RING ADVANCED IN ONE JUMP    THEY MUST NOT SEPARATE",
    );

    let readout = format!(
        "SPEED {speed:.2}X    WORST DRIFT {} PX",
        format!("{worst_drift:.2e}").to_uppercase()
    );
    canvas.text(
        WIDTH - MARGIN - font::width(&readout, 1),
        46,
        1,
        TEXT_DIM,
        &readout,
    );

    canvas.rect(MARGIN, 62, WIDTH - 2 * MARGIN, 1, GRID);
}

fn draw_lanes(canvas: &mut Canvas, lanes: &[Lane], target: f64) {
    // One target line behind every lane.
    canvas.dashed_column(
        target.round() as i32,
        LANES_TOP + 6,
        LANES_TOP + LANE_COUNT * LANE_HEIGHT - 6,
        5,
        4,
        TARGET_INK,
    );

    for (index, lane) in lanes.iter().enumerate() {
        let center = f64::from(Lane::center_y(index));
        let top = Lane::center_y(index);

        canvas.rect(TRACK_LEFT, top, TRACK_RIGHT - TRACK_LEFT, 1, RAIL);

        canvas.text(MARGIN, top - 28, 2, lane.color, lane.label);
        canvas.text(
            MARGIN,
            top - 8,
            1,
            TEXT_DIM,
            &format!(
                "ZETA {:.2}  W0 {:.1}",
                lane.spring.config().damping_ratio(),
                lane.spring.config().angular_frequency()
            ),
        );
        canvas.text(
            MARGIN,
            top + 4,
            1,
            TEXT_DIM,
            &format!("VEL {:.0}", lane.spring.velocity()),
        );

        // The single-jump reference first, so the frame-stepped puck sits on
        // top of it and any disagreement shows as a crescent.
        let reference = lane.reference();
        canvas.ring(reference.value(), center, PUCK_RADIUS + 4.0, 1.5, GHOST_INK);
        canvas.disc(lane.spring.value(), center, PUCK_RADIUS, lane.color);
    }
}

fn draw_trace(canvas: &mut Canvas, lanes: &[Lane], target_trace: &VecDeque<f64>) {
    canvas.rect(
        MARGIN,
        TRACE_TOP,
        WIDTH - 2 * MARGIN,
        TRACE_BOTTOM - TRACE_TOP,
        PANEL,
    );
    canvas.outline(
        MARGIN,
        TRACE_TOP,
        WIDTH - 2 * MARGIN,
        TRACE_BOTTOM - TRACE_TOP,
        GRID,
    );
    canvas.text(
        MARGIN + 12,
        TRACE_TOP + 12,
        1,
        TEXT_DIM,
        "POSITION OVER TIME    OLDEST ON THE LEFT",
    );

    // The target, so overshoot is a crossing rather than a judgement call.
    for (sample, value) in target_trace.iter().enumerate() {
        let x = TRACK_LEFT + sample as i32;
        canvas.rect(x, trace_y(*value).round() as i32, 1, 1, TARGET_INK);
    }

    for lane in lanes {
        let mut previous: Option<f64> = None;

        for (sample, value) in lane.trace.iter().enumerate() {
            let x = TRACK_LEFT + sample as i32;
            let y = trace_y(*value);

            // Join consecutive samples with a vertical run so fast motion
            // stays a continuous curve instead of a dotted one.
            let from = previous.unwrap_or(y);
            let (top, bottom) = (from.min(y), from.max(y));
            canvas.rect(
                x,
                top.round() as i32,
                1,
                (bottom - top).round() as i32 + 1,
                lane.color,
            );

            previous = Some(y);
        }
    }
}

/// Maps a spring value (a pixel column on the track) to a row in the trace.
fn trace_y(value: f64) -> f64 {
    let normalized = ((value - TRACK_MIN) / (TRACK_MAX - TRACK_MIN)).clamp(-0.08, 1.08);
    let span = f64::from(TRACE_BOTTOM - TRACE_TOP) - 2.0 * TRACE_PADDING;

    f64::from(TRACE_BOTTOM) - TRACE_PADDING - normalized * span
}

fn draw_footer(canvas: &mut Canvas) {
    canvas.text(
        MARGIN,
        HEIGHT - font::GLYPH_HEIGHT - 14,
        1,
        TEXT_DIM,
        "CLICK OR DRAG RETARGET    SPACE RANDOM    V KICK VELOCITY    R RESET    1/2/3 SPEED    ESC QUIT",
    );
}

// ---------------------------------------------------------------------------
// Random
// ---------------------------------------------------------------------------

/// xorshift64*, so the demo does not need a dependency for two random numbers.
struct Random(u64);

impl Random {
    fn seeded() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.subsec_nanos() as u64)
            .unwrap_or(1);

        Self(nanos | 1)
    }

    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;

        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}
