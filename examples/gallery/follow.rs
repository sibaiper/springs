//! A chain of two-dimensional springs chasing the mouse.
//!
//! Every node is a `Spring<[f64; 2]>` — the const-generic array impl, no
//! geometry library involved. Node 0 targets the cursor and each node after it
//! targets the one in front, which is the whole implementation of a trailing
//! whip: fourteen `set_target` calls and fourteen `advance` calls per frame.
//!
//! The arrow keys retune every node live, which makes the two config knobs
//! legible in a way a static curve never is: bounce is the overshoot in the
//! tail, duration is how far behind the cursor the chain lags.

use minifb::Key;
use springs::{Spring, SpringConfig};

use crate::canvas::{self, Canvas};
use crate::{ACCENT, CONTENT_BOTTOM, CONTENT_TOP, Input, MARGIN, PURPLE, TEXT, TEXT_DIM, WIDTH};

const NODES: usize = 14;
const HEAD_RADIUS: f64 = 17.0;
const TAIL_RADIUS: f64 = 4.0;

pub struct Follow {
    nodes: Vec<Spring<[f64; 2]>>,
    duration: f64,
    bounce: f64,
    trail: Vec<(f64, f64)>,
}

impl Follow {
    pub const HELP: &'static str =
        "MOVE THE MOUSE   UP/DOWN: BOUNCE   LEFT/RIGHT: DURATION   R: RESET";

    pub fn new() -> Self {
        let centre = [
            f64::from(WIDTH) / 2.0,
            f64::from(CONTENT_TOP + CONTENT_BOTTOM) / 2.0,
        ];

        let mut follow = Self {
            nodes: Vec::new(),
            duration: 0.32,
            bounce: 0.35,
            trail: Vec::new(),
        };
        follow.nodes = (0..NODES)
            .map(|_| Spring::new(centre).with_config(follow.config()))
            .collect();

        follow
    }

    fn config(&self) -> SpringConfig {
        SpringConfig::new()
            .duration(self.duration)
            .bounce(self.bounce)
    }

    fn retune(&mut self) {
        let config = self.config();
        for node in &mut self.nodes {
            *node = node.with_config(config);
        }
    }

    pub fn update(&mut self, input: &Input) {
        let before = (self.duration, self.bounce);

        if input.down(Key::Up) {
            self.bounce = (self.bounce + 0.7 * input.dt).min(0.95);
        }
        if input.down(Key::Down) {
            self.bounce = (self.bounce - 0.7 * input.dt).max(-0.9);
        }
        if input.down(Key::Right) {
            self.duration = (self.duration + 0.5 * input.dt).min(1.5);
        }
        if input.down(Key::Left) {
            self.duration = (self.duration - 0.5 * input.dt).max(0.06);
        }
        if before != (self.duration, self.bounce) {
            self.retune();
        }

        if input.pressed(Key::R) {
            *self = Self {
                duration: self.duration,
                bounce: self.bounce,
                ..Self::new()
            };
            self.retune();
        }

        // The cursor, kept inside the content area so the chain stays visible.
        let mut target = [
            input
                .mouse
                .0
                .clamp(f64::from(MARGIN), f64::from(WIDTH - MARGIN)),
            input.mouse.1.clamp(
                f64::from(CONTENT_TOP) + HEAD_RADIUS,
                f64::from(CONTENT_BOTTOM) - HEAD_RADIUS,
            ),
        ];

        self.trail.push((target[0], target[1]));
        if self.trail.len() > 90 {
            self.trail.remove(0);
        }

        // Each node chases the one ahead of it, using that node's *new*
        // position, so the whole chain resolves in a single pass.
        for node in &mut self.nodes {
            node.set_target(target);
            node.advance(input.dt);
            target = node.value();
        }
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        crate::caption(
            canvas,
            MARGIN,
            CONTENT_TOP + 4,
            "SPRING<[F64; 2]>",
            "FOURTEEN NODES, EACH CHASING THE ONE IN FRONT",
        );

        // Where the cursor has actually been, for comparison with where the
        // chain thinks it is.
        for pair in self.trail.windows(2) {
            canvas.line(pair[0], pair[1], 1.0, crate::RAIL);
        }

        // Back to front, so the head sits on top.
        for index in (0..self.nodes.len()).rev() {
            let amount = index as f64 / (NODES - 1) as f64;
            let colour = canvas::mix(ACCENT, PURPLE, amount);
            let radius = HEAD_RADIUS + (TAIL_RADIUS - HEAD_RADIUS) * amount;

            let [x, y] = self.nodes[index].value();

            if index + 1 < self.nodes.len() {
                let [nx, ny] = self.nodes[index + 1].value();
                canvas.line((x, y), (nx, ny), radius * 0.75, colour);
            }

            canvas.disc(x, y, radius, colour);
        }

        let readout = format!(
            "DURATION {:.2}S   BOUNCE {:+.2}   ZETA {:.2}   W0 {:.1}",
            self.duration,
            self.bounce,
            self.config().damping_ratio(),
            self.config().angular_frequency()
        );
        canvas.text(MARGIN, CONTENT_BOTTOM - 26, 1, TEXT, &readout);
        canvas.text(
            MARGIN,
            CONTENT_BOTTOM - 12,
            1,
            TEXT_DIM,
            "THE FAINT LINE IS WHERE THE CURSOR ACTUALLY WENT",
        );
    }
}
