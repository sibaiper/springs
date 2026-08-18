//! The whole config space at once: twenty five springs, one grid.
//!
//! Every cell is a live `Spring<f64>` on a different duration and bounce,
//! stepping in lockstep with the other twenty four and drawing the step
//! response it has produced since the last retarget. Reading down a column
//! shows what bounce does at a fixed speed; reading across a row shows what
//! duration does at a fixed shape.
//!
//! Two things are easier to see here than in any single curve. The row at
//! bounce 0.00 is the boundary: everything above it crosses the target, nothing
//! below it does. And in the default view every cell shares one time axis, so a
//! 0.15 s spring really is finished in a tenth of the space a 1.2 s spring
//! needs — press N to give each cell its own axis instead, which makes the
//! shapes comparable but hides the speed.
//!
//! ```text
//! cargo run --example matrix --release
//! ```

#[path = "../common/canvas.rs"]
mod canvas;
#[path = "../common/font.rs"]
mod font;

use std::time::Instant;

use canvas::Canvas;
use minifb::{Key, KeyRepeat, MouseMode, Window, WindowOptions};
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

const DURATIONS: [f64; 5] = [0.15, 0.3, 0.5, 0.8, 1.2];
const BOUNCES: [f64; 5] = [-0.4, 0.0, 0.25, 0.5, 0.75];

const GRID_TOP: i32 = 128;
const GRID_BOTTOM: i32 = HEIGHT - 76;

/// Seconds of history every cell shows when they share one time axis.
const SHARED_WINDOW: f64 = 2.6;
/// Multiples of a cell's own duration when each cell gets its own axis.
const OWN_WINDOW: f64 = 4.5;

struct Cell {
    duration: f64,
    bounce: f64,
    spring: Spring<f64>,
    /// (seconds since the last retarget, value).
    trace: Vec<(f64, f64)>,
    elapsed: f64,
}

impl Cell {
    fn config(&self) -> SpringConfig {
        self.spring.config()
    }

    fn window(&self, normalised: bool) -> f64 {
        if normalised {
            self.duration * OWN_WINDOW
        } else {
            SHARED_WINDOW
        }
    }

    /// The theoretical first-peak overshoot, as a fraction of the travel.
    fn overshoot(&self) -> f64 {
        let zeta = self.config().damping_ratio();
        if zeta >= 1.0 {
            return 0.0;
        }

        (-std::f64::consts::PI * zeta / (1.0 - zeta * zeta).sqrt()).exp()
    }

    fn colour(&self) -> u32 {
        if self.bounce < 0.0 {
            canvas::mix(PURPLE, GREEN, (self.bounce + 0.4) / 0.4)
        } else {
            canvas::mix(GREEN, ORANGE, self.bounce / 0.75)
        }
    }
}

struct Matrix {
    cells: Vec<Cell>,
    high: bool,
    clock: f64,
    normalised: bool,
    hovered: Option<usize>,
}

fn cell_rect(index: usize) -> (f64, f64, f64, f64) {
    let columns = DURATIONS.len() as f64;
    let rows = BOUNCES.len() as f64;

    let width = f64::from(WIDTH - 2 * MARGIN) / columns;
    let height = f64::from(GRID_BOTTOM - GRID_TOP) / rows;

    let column = (index % DURATIONS.len()) as f64;
    let row = (index / DURATIONS.len()) as f64;

    (
        f64::from(MARGIN) + column * width,
        f64::from(GRID_TOP) + row * height,
        width,
        height,
    )
}

impl Matrix {
    fn new() -> Self {
        let cells = BOUNCES
            .iter()
            .flat_map(|&bounce| {
                DURATIONS.iter().map(move |&duration| Cell {
                    duration,
                    bounce,
                    spring: Spring::new(0.0)
                        .with_config(SpringConfig::new().duration(duration).bounce(bounce))
                        .with_epsilon(0.0004),
                    trace: Vec::new(),
                    elapsed: 0.0,
                })
            })
            .collect();

        Self {
            cells,
            high: false,
            clock: 0.0,
            normalised: false,
            hovered: None,
        }
    }

    fn cycle(&self) -> f64 {
        if self.normalised {
            DURATIONS[DURATIONS.len() - 1] * OWN_WINDOW + 0.7
        } else {
            SHARED_WINDOW + 0.7
        }
    }

    fn retarget(&mut self) {
        self.high = !self.high;
        self.clock = 0.0;

        let target = if self.high { 1.0 } else { 0.0 };
        for cell in &mut self.cells {
            cell.spring.set_target(target);
            cell.trace.clear();
            cell.elapsed = 0.0;
        }
    }

    fn update(&mut self, mouse: (f64, f64), dt: f64, retarget: bool, toggle: bool) {
        if toggle {
            self.normalised = !self.normalised;
            self.clock = self.cycle(); // start a fresh cycle on the new axis
        }

        self.clock += dt;
        if retarget || self.clock >= self.cycle() {
            self.retarget();
        }

        self.hovered = (0..self.cells.len()).find(|&index| {
            let (x, y, width, height) = cell_rect(index);
            (x..x + width).contains(&mouse.0) && (y..y + height).contains(&mouse.1)
        });

        for cell in &mut self.cells {
            cell.spring.advance(dt);
            cell.elapsed += dt;
            cell.trace.push((cell.elapsed, cell.spring.value()));
        }
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.text(MARGIN, 22, 2, TEXT_BRIGHT, "SPRINGS");
        canvas.text(
            MARGIN + font::width("SPRINGS", 2) + 16,
            26,
            1,
            TEXT_DIM,
            "MATRIX - DURATION ACROSS, BOUNCE DOWN",
        );
        canvas.rect(MARGIN, 46, WIDTH - 2 * MARGIN, 1, PANEL_EDGE);

        canvas.text(
            MARGIN,
            62,
            1,
            TEXT,
            if self.normalised {
                "EACH CELL ON ITS OWN TIME AXIS - SHAPES COMPARABLE, SPEED HIDDEN"
            } else {
                "ALL CELLS ON ONE TIME AXIS - A 0.15S SPRING FINISHES IN A TENTH OF THE SPACE"
            },
        );

        // Column headings.
        for (column, duration) in DURATIONS.iter().enumerate() {
            let (x, _, width, _) = cell_rect(column);
            let label = format!("{duration:.2}S");
            canvas.text(
                (x + (width - f64::from(font::width(&label, 2))) / 2.0) as i32,
                GRID_TOP - 26,
                2,
                TEXT_BRIGHT,
                &label,
            );
        }

        for (index, cell) in self.cells.iter().enumerate() {
            self.draw_cell(canvas, index, cell);
        }

        self.draw_footer(canvas);
    }

    fn draw_cell(&self, canvas: &mut Canvas, index: usize, cell: &Cell) {
        let (x, y, width, height) = cell_rect(index);
        let hovered = self.hovered == Some(index);

        let inset = 5.0;
        let (px, py) = (x + inset, y + inset);
        let (pw, ph) = (width - inset * 2.0, height - inset * 2.0);

        canvas.rounded_rect(px, py, pw, ph, 8.0, PANEL);
        if hovered {
            canvas.rounded_rect(px, py, pw, 2.0, 1.0, cell.colour());
        }

        // Plot area inside the cell, with room above the target line for
        // overshoot to have somewhere to go.
        let plot_top = py + 28.0;
        let plot_bottom = py + ph - 12.0;
        let zero = plot_bottom;
        let one = plot_top + (plot_bottom - plot_top) * 0.38;

        canvas.rect(px as i32 + 10, one as i32, pw as i32 - 20, 1, RAIL);
        canvas.rect(px as i32 + 10, zero as i32, pw as i32 - 20, 1, RAIL);

        let window = cell.window(self.normalised);
        let to_screen = |(t, value): (f64, f64)| {
            (
                px + 10.0 + (t / window) * (pw - 20.0),
                zero + (one - zero) * value,
            )
        };

        let mut previous: Option<(f64, f64)> = None;
        for sample in &cell.trace {
            if sample.0 > window {
                break;
            }

            let point = to_screen(*sample);
            if let Some(from) = previous {
                canvas.line(from, point, 1.8, cell.colour());
            }
            previous = Some(point);
        }

        if let Some(head) = previous {
            canvas.disc(head.0, head.1, 3.2, TEXT_BRIGHT);
        }

        let config = cell.config();
        canvas.text(
            px as i32 + 10,
            py as i32 + 9,
            1,
            if hovered { TEXT_BRIGHT } else { TEXT },
            &format!("BOUNCE {:+.2}", cell.bounce),
        );
        canvas.text(
            px as i32 + 10,
            py as i32 + 20,
            1,
            TEXT_DIM,
            &format!(
                "ZETA {:.2}  OVER {:.0}%",
                config.damping_ratio(),
                cell.overshoot() * 100.0
            ),
        );
    }

    fn draw_footer(&self, canvas: &mut Canvas) {
        canvas.rect(MARGIN, GRID_BOTTOM + 10, WIDTH - 2 * MARGIN, 1, PANEL_EDGE);

        let detail = match self.hovered.map(|index| &self.cells[index]) {
            Some(cell) => {
                let config = cell.config();
                format!(
                    "DURATION {:.2}S   BOUNCE {:+.2}   ZETA {:.3}   W0 {:.2} RAD/S   OVERSHOOT {:.1}%   VALUE {:.4}",
                    cell.duration,
                    cell.bounce,
                    config.damping_ratio(),
                    config.angular_frequency(),
                    cell.overshoot() * 100.0,
                    cell.spring.value(),
                )
            }
            None => "HOVER A CELL FOR ITS NUMBERS".to_string(),
        };
        canvas.text(MARGIN, GRID_BOTTOM + 26, 1, TEXT, &detail);

        canvas.text(
            MARGIN,
            HEIGHT - 22,
            1,
            TEXT_DIM,
            "SPACE: RETARGET NOW   N: SHARED OR PER CELL TIME AXIS   ESC: QUIT",
        );

        let target = format!("TARGET {}", if self.high { "1" } else { "0" });
        canvas.text(
            WIDTH - MARGIN - font::width(&target, 1),
            HEIGHT - 22,
            1,
            ACCENT,
            &target,
        );
    }
}

fn main() -> Result<(), minifb::Error> {
    let mut window = Window::new(
        "springs - matrix",
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions::default(),
    )?;
    window.set_target_fps(120);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut matrix = Matrix::new();
    let mut clock = Instant::now();

    matrix.retarget();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = clock.elapsed().as_secs_f64().min(0.1);
        clock = Instant::now();

        let mouse = window
            .get_mouse_pos(MouseMode::Pass)
            .map(|(x, y)| (f64::from(x), f64::from(y)))
            .unwrap_or((-1.0, -1.0));

        matrix.update(
            mouse,
            dt,
            window.is_key_pressed(Key::Space, KeyRepeat::No),
            window.is_key_pressed(Key::N, KeyRepeat::No),
        );

        canvas.clear(BACKGROUND);
        matrix.draw(&mut canvas);

        window.update_with_buffer(canvas.buffer(), WIDTH as usize, HEIGHT as usize)?;
    }

    Ok(())
}
