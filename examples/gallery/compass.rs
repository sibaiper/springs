//! Two needles springing to the same heading, one of them wrong on purpose.
//!
//! The bright needle is a `Spring<Angle>`: its `displacement_from` wraps into
//! (−π, π], so 359° → 2° is a 3° nudge. The dim red one is a plain
//! `Spring<f64>` over raw degrees, which is what you get if you forget that
//! angles are circular — it sails 357° the other way round to reach the same
//! place.
//!
//! Both needles accumulate the distance they travel, which turns the
//! difference into a number rather than an argument.

use minifb::Key;
use springs::{Angle, Spring, SpringConfig, SpringValue};

use crate::canvas::Canvas;
use crate::{
    ACCENT, CONTENT_BOTTOM, CONTENT_TOP, Input, MARGIN, PANEL_EDGE, RAIL, RED, TEXT, TEXT_BRIGHT,
    TEXT_DIM, WIDTH,
};

pub struct Compass {
    wrapped: Spring<Angle>,
    /// The same animation over raw degrees, with no notion of wrapping.
    naive: Spring<f64>,

    target: f64,
    wrapped_travel: f64,
    naive_travel: f64,

    last_wrapped: f64,
    last_naive: f64,
}

fn config() -> SpringConfig {
    SpringConfig::new().duration(0.9).bounce(0.15)
}

fn centre() -> (f64, f64) {
    (
        f64::from(WIDTH) / 2.0,
        f64::from(CONTENT_TOP + CONTENT_BOTTOM) / 2.0 + 10.0,
    )
}

const DIAL_RADIUS: f64 = 218.0;

/// Screen position of a point `radius` out along `degrees`, measured clockwise
/// from straight up.
fn along(degrees: f64, radius: f64) -> (f64, f64) {
    let (cx, cy) = centre();
    let radians = degrees.to_radians();

    (cx + radians.sin() * radius, cy - radians.cos() * radius)
}

impl Compass {
    pub const HELP: &'static str =
        "CLICK THE DIAL TO SET A HEADING   SPACE: OPPOSITE SIDE   R: RESET";

    pub fn new() -> Self {
        Self {
            wrapped: Spring::new(Angle::from_degrees(0.0)).with_config(config()),
            naive: Spring::new(0.0).with_config(config()),
            target: 0.0,
            wrapped_travel: 0.0,
            naive_travel: 0.0,
            last_wrapped: 0.0,
            last_naive: 0.0,
        }
    }

    fn retarget(&mut self, degrees: f64) {
        let degrees = degrees.rem_euclid(360.0);

        self.target = degrees;
        self.wrapped.set_target(Angle::from_degrees(degrees));
        self.naive.set_target(degrees);

        self.wrapped_travel = 0.0;
        self.naive_travel = 0.0;
    }

    pub fn update(&mut self, input: &Input) {
        if input.clicked {
            let (cx, cy) = centre();
            let (dx, dy) = (input.mouse.0 - cx, cy - input.mouse.1);

            if dx.hypot(dy) > 12.0 {
                self.retarget(dx.atan2(dy).to_degrees());
            }
        }

        if input.pressed(Key::Space) {
            self.retarget(self.target + 174.0);
        }

        if input.pressed(Key::R) {
            *self = Self::new();
        }

        self.wrapped.advance(input.dt);
        self.naive.advance(input.dt);

        // Distance actually covered, wrapping each step so a 359 → 0 crossing
        // counts as one degree rather than three hundred and fifty nine.
        let wrapped = self.wrapped.value().degrees();
        let step = (wrapped - self.last_wrapped + 540.0).rem_euclid(360.0) - 180.0;
        self.wrapped_travel += step.abs();
        self.last_wrapped = wrapped;

        let naive = self.naive.value();
        self.naive_travel += (naive - self.last_naive).abs();
        self.last_naive = naive;
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        let (cx, cy) = centre();

        crate::caption(
            canvas,
            MARGIN,
            CONTENT_TOP + 4,
            "SPRING<ANGLE>",
            "WRAPPED AGAINST NAIVE, ON THE SAME CONFIG",
        );

        canvas.ring(cx, cy, DIAL_RADIUS, 1.5, PANEL_EDGE);
        canvas.ring(cx, cy, DIAL_RADIUS - 34.0, 1.0, RAIL);

        for step in 0..36 {
            let degrees = f64::from(step) * 10.0;
            let major = step % 9 == 0;

            let inner = if major { 22.0 } else { 10.0 };
            canvas.line(
                along(degrees, DIAL_RADIUS - inner),
                along(degrees, DIAL_RADIUS - 2.0),
                if major { 2.0 } else { 1.0 },
                if major { TEXT_DIM } else { RAIL },
            );
        }

        for (degrees, label) in [(0.0, "N"), (90.0, "E"), (180.0, "S"), (270.0, "W")] {
            let (x, y) = along(degrees, DIAL_RADIUS - 44.0);
            canvas.text(
                x.round() as i32 - 5,
                y.round() as i32 - 7,
                2,
                TEXT_DIM,
                label,
            );
        }

        // Where it is heading.
        canvas.line(
            along(self.target, DIAL_RADIUS - 30.0),
            along(self.target, DIAL_RADIUS - 4.0),
            3.0,
            TEXT_BRIGHT,
        );

        // The wrong needle first, so the right one draws over it.
        canvas.line(
            (cx, cy),
            along(self.naive.value(), DIAL_RADIUS - 58.0),
            3.0,
            RED,
        );
        canvas.line(
            (cx, cy),
            along(self.wrapped.value().degrees(), DIAL_RADIUS - 42.0),
            5.0,
            ACCENT,
        );
        canvas.disc(cx, cy, 9.0, TEXT_BRIGHT);

        // Readouts.
        let displacement = self
            .wrapped
            .value()
            .displacement_from(Angle::from_degrees(self.target))
            .to_degrees();

        let rows = [
            (
                ACCENT,
                format!("ANGLE      {:>7.1} DEG", self.wrapped.value().degrees()),
            ),
            (
                ACCENT,
                format!("TO TARGET  {displacement:>+7.1} DEG   (ALWAYS WITHIN 180)"),
            ),
            (
                ACCENT,
                format!("TRAVELLED  {:>7.1} DEG", self.wrapped_travel),
            ),
            (RED, format!("NAIVE      {:>7.1} DEG", self.naive.value())),
            (RED, format!("TRAVELLED  {:>7.1} DEG", self.naive_travel)),
        ];

        for (index, (ink, row)) in rows.iter().enumerate() {
            canvas.text(MARGIN, CONTENT_TOP + 74 + index as i32 * 16, 1, *ink, row);
        }

        canvas.text(
            MARGIN,
            CONTENT_BOTTOM - 12,
            1,
            TEXT,
            "TRY A HEADING JUST ACROSS NORTH: THE RED NEEDLE GOES ALL THE WAY ROUND",
        );
    }
}
