//! The same release, plotted three ways.
//!
//! The left panel is a phase portrait: displacement across, velocity up. It is
//! the clearest picture of what the damping ratio actually does. An underdamped
//! spring spirals into the origin, because it keeps trading position for
//! velocity on the way in; a critically damped one runs straight down to the
//! origin and stops; an overdamped one takes the same path more slowly.
//!
//! Velocity is drawn as v / ω₀ so the axes share units — with that scaling an
//! undamped spring would trace an exact circle, and everything else is that
//! circle decaying.

use minifb::Key;
use springs::{Spring, SpringConfig};

use crate::canvas::{self, Canvas};
use crate::{
    ACCENT, CONTENT_BOTTOM, CONTENT_TOP, GREEN, Input, MARGIN, ORANGE, PANEL, PANEL_EDGE, RAIL,
    TEXT, TEXT_BRIGHT, TEXT_DIM, WIDTH,
};

const RESPONSE: f64 = 0.9;
const TRAIL: usize = 1_400;
const HISTORY: usize = 520;

const PLOT: f64 = 400.0;
const PLOT_LEFT: f64 = 60.0;

struct Trace {
    label: &'static str,
    colour: u32,
    spring: Spring<f64>,
    trail: Vec<(f64, f64)>,
    history: Vec<f64>,
}

pub struct Phase {
    traces: Vec<Trace>,
    start: (f64, f64),
    idle: f64,
}

fn plot_top() -> f64 {
    f64::from(CONTENT_TOP) + 60.0
}

impl Phase {
    pub const HELP: &'static str =
        "CLICK THE PORTRAIT TO RELEASE FROM THERE   SPACE: REPLAY   R: RESET";

    pub fn new() -> Self {
        let mut phase = Self {
            traces: [
                ("UNDERDAMPED  ZETA 0.15", ACCENT, 0.15),
                ("CRITICAL     ZETA 1.00", GREEN, 1.0),
                ("OVERDAMPED   ZETA 2.20", ORANGE, 2.2),
            ]
            .into_iter()
            .map(|(label, colour, zeta)| Trace {
                label,
                colour,
                spring: Spring::new(0.0)
                    .with_config(SpringConfig::from_response_damping(RESPONSE, zeta))
                    .with_epsilon(0.002),
                trail: Vec::with_capacity(TRAIL),
                history: Vec::with_capacity(HISTORY),
            })
            .collect(),
            start: (0.85, 0.0),
            idle: 0.0,
        };
        phase.release(phase.start);

        phase
    }

    /// Restarts every trace from the same displacement and velocity.
    fn release(&mut self, (displacement, scaled_velocity): (f64, f64)) {
        self.start = (displacement, scaled_velocity);
        self.idle = 0.0;

        for trace in &mut self.traces {
            let omega = trace.spring.config().angular_frequency();

            trace.spring.snap_to(displacement);
            trace.spring.set_target(0.0);
            trace.spring.set_velocity(scaled_velocity * omega);
            trace.trail.clear();
            trace.history.clear();
        }
    }

    pub fn update(&mut self, input: &Input) {
        if input.clicked {
            let x = (input.mouse.0 - (PLOT_LEFT + PLOT / 2.0)) / (PLOT / 2.0);
            let y = ((plot_top() + PLOT / 2.0) - input.mouse.1) / (PLOT / 2.0);

            if x.abs() <= 1.05 && y.abs() <= 1.05 {
                self.release((x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0)));
            }
        }

        if input.pressed(Key::Space) {
            self.release(self.start);
        }

        if input.pressed(Key::R) {
            *self = Self::new();
        }

        let mut all_settled = true;
        for trace in &mut self.traces {
            trace.spring.advance(input.dt);

            let omega = trace.spring.config().angular_frequency();
            trace
                .trail
                .push((trace.spring.value(), trace.spring.velocity() / omega));
            if trace.trail.len() > TRAIL {
                trace.trail.remove(0);
            }

            trace.history.push(trace.spring.value());
            if trace.history.len() > HISTORY {
                trace.history.remove(0);
            }

            all_settled &= trace.spring.is_settled();
        }

        // Never leave the scene static: once everything has arrived, wait a
        // beat and release again.
        if all_settled {
            self.idle += input.dt;
            if self.idle > 1.4 {
                self.release(self.start);
            }
        }
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        crate::caption(
            canvas,
            MARGIN,
            CONTENT_TOP + 4,
            "PHASE PORTRAIT",
            "DISPLACEMENT ACROSS, VELOCITY / W0 UP — THE SAME RELEASE IN THREE REGIMES",
        );

        self.draw_portrait(canvas);
        self.draw_history(canvas);
    }

    fn draw_portrait(&self, canvas: &mut Canvas) {
        let top = plot_top();
        let (cx, cy) = (PLOT_LEFT + PLOT / 2.0, top + PLOT / 2.0);

        canvas.rect(
            PLOT_LEFT as i32,
            top as i32,
            PLOT as i32,
            PLOT as i32,
            PANEL,
        );
        canvas.outline(
            PLOT_LEFT as i32,
            top as i32,
            PLOT as i32,
            PLOT as i32,
            PANEL_EDGE,
        );

        // Unit circle: the orbit an undamped spring would keep forever.
        canvas.ring(cx, cy, PLOT / 2.0 - 20.0, 1.0, RAIL);
        canvas.rect(PLOT_LEFT as i32, cy as i32, PLOT as i32, 1, RAIL);
        canvas.rect(cx as i32, top as i32, 1, PLOT as i32, RAIL);

        let to_screen = |(displacement, velocity): (f64, f64)| {
            (
                cx + displacement * (PLOT / 2.0 - 20.0),
                cy - velocity * (PLOT / 2.0 - 20.0),
            )
        };

        for trace in &self.traces {
            for (index, pair) in trace.trail.windows(2).enumerate() {
                // Older segments fade into the panel.
                let age = index as f64 / trace.trail.len().max(2) as f64;
                let colour = canvas::mix(PANEL, trace.colour, 0.25 + 0.75 * age);

                canvas.line(to_screen(pair[0]), to_screen(pair[1]), 1.8, colour);
            }

            if let Some(head) = trace.trail.last() {
                let (x, y) = to_screen(*head);
                canvas.disc(x, y, 4.5, trace.colour);
            }
        }

        canvas.disc(cx, cy, 3.0, TEXT_BRIGHT);
        canvas.text(
            (cx + 8.0) as i32,
            (cy + 6.0) as i32,
            1,
            TEXT_DIM,
            "TARGET, AT REST",
        );
    }

    fn draw_history(&self, canvas: &mut Canvas) {
        let left = PLOT_LEFT + PLOT + 56.0;
        let width = f64::from(WIDTH - MARGIN) - left;
        let top = plot_top();
        let height = PLOT;

        canvas.rect(left as i32, top as i32, width as i32, height as i32, PANEL);
        canvas.outline(
            left as i32,
            top as i32,
            width as i32,
            height as i32,
            PANEL_EDGE,
        );

        let centre = top + height / 2.0;
        canvas.rect(left as i32, centre as i32, width as i32, 1, RAIL);
        canvas.text(
            left as i32 + 12,
            top as i32 + 12,
            1,
            TEXT_DIM,
            "THE SAME MOTION AGAINST TIME",
        );

        for trace in &self.traces {
            let mut previous: Option<(f64, f64)> = None;

            for (index, value) in trace.history.iter().enumerate() {
                let x = left + index as f64 * (width / HISTORY as f64);
                let y = centre - value * (height / 2.0 - 24.0);

                if let Some(point) = previous {
                    canvas.line(point, (x, y), 1.6, trace.colour);
                }
                previous = Some((x, y));
            }
        }

        for (index, trace) in self.traces.iter().enumerate() {
            let y = CONTENT_BOTTOM - 58 + index as i32 * 16;
            canvas.rect(left as i32, y + 2, 10, 3, trace.colour);
            canvas.text(left as i32 + 18, y, 1, TEXT, trace.label);
        }
    }
}
