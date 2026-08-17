//! What the crate is actually for: four ordinary interface animations.
//!
//! Each one uses a different config, chosen for its job rather than for
//! variety — a tab indicator wants no overshoot because it has to line up with
//! a label, a toggle wants a little, a sheet wants more, and a staggered list
//! wants the same spring started at different times.
//!
//! The tab pill is the one worth looking at twice: it is a single
//! `Spring<[f64; 2]>` animating position and width together, so the pill
//! stretches as it travels instead of sliding rigidly.

use minifb::Key;
use springs::{Spring, SpringConfig};

use crate::canvas::{self, Canvas};
use crate::{
    ACCENT, BACKGROUND, CONTENT_BOTTOM, CONTENT_TOP, GREEN, Input, MARGIN, PANEL, PANEL_EDGE, RAIL,
    TEXT, TEXT_BRIGHT, TEXT_DIM, WIDTH,
};

const TABS: [&str; 4] = ["OVERVIEW", "ACTIVITY", "REPORTS", "SETTINGS"];
const ROWS: usize = 6;

const TAB_BAR_X: f64 = 60.0;
const TAB_BAR_WIDTH: f64 = 470.0;
const TAB_HEIGHT: f64 = 40.0;

pub struct Interface {
    tab: usize,
    /// Position and width together, so the pill stretches as it moves.
    pill: Spring<[f64; 2]>,

    on: bool,
    knob: Spring<f64>,

    sheet_open: bool,
    sheet: Spring<f64>,

    rows: Vec<Spring<f64>>,
    delays: Vec<f64>,
    released: Vec<bool>,
    stagger_clock: f64,
}

fn tab_slot(index: usize) -> [f64; 2] {
    let width = TAB_BAR_WIDTH / TABS.len() as f64;

    [TAB_BAR_X + width * index as f64, width]
}

fn tab_bar_y() -> f64 {
    f64::from(CONTENT_TOP) + 74.0
}

fn toggle_y() -> f64 {
    f64::from(CONTENT_TOP) + 196.0
}

fn list_top() -> f64 {
    f64::from(CONTENT_TOP) + 300.0
}

impl Interface {
    pub const HELP: &'static str =
        "CLICK A TAB OR THE TOGGLE   S: SHEET   L: REPLAY THE LIST   R: RESET";

    pub fn new() -> Self {
        let delays: Vec<f64> = (0..ROWS).map(|row| row as f64 * 0.055).collect();

        Self {
            tab: 0,
            // No bounce: a tab indicator has to line up with its label.
            pill: Spring::new(tab_slot(0))
                .with_config(SpringConfig::new().duration(0.42).bounce(0.0))
                .with_epsilon(0.05),

            on: false,
            knob: Spring::new(0.0)
                .with_config(SpringConfig::new().duration(0.3).bounce(0.45))
                .with_epsilon(0.0005),

            sheet_open: false,
            sheet: Spring::new(0.0)
                .with_config(SpringConfig::new().duration(0.55).bounce(0.3))
                .with_epsilon(0.0005),

            rows: (0..ROWS)
                .map(|_| {
                    Spring::new(1.0)
                        .with_target(0.0)
                        .with_config(SpringConfig::new().duration(0.5).bounce(0.25))
                        .with_epsilon(0.0005)
                })
                .collect(),
            delays,
            released: vec![true; ROWS],
            stagger_clock: 10.0,
        }
    }

    fn replay_list(&mut self) {
        for row in &mut self.rows {
            row.snap_to(1.0);
            row.set_target(0.0);
        }
        self.released = vec![false; ROWS];
        self.stagger_clock = 0.0;
    }

    pub fn update(&mut self, input: &Input) {
        if input.clicked {
            let (x, y) = input.mouse;

            // Tabs.
            if (tab_bar_y()..tab_bar_y() + TAB_HEIGHT).contains(&y)
                && (TAB_BAR_X..TAB_BAR_X + TAB_BAR_WIDTH).contains(&x)
            {
                let width = TAB_BAR_WIDTH / TABS.len() as f64;
                self.tab = (((x - TAB_BAR_X) / width) as usize).min(TABS.len() - 1);
                self.pill.set_target(tab_slot(self.tab));
            }

            // Toggle.
            if (toggle_y()..toggle_y() + 34.0).contains(&y) && (60.0..=176.0).contains(&x) {
                self.on = !self.on;
                self.knob.set_target(if self.on { 1.0 } else { 0.0 });
            }
        }

        if input.pressed(Key::S) {
            self.sheet_open = !self.sheet_open;
            self.sheet
                .set_target(if self.sheet_open { 1.0 } else { 0.0 });
        }

        if input.pressed(Key::L) {
            self.replay_list();
        }

        if input.pressed(Key::R) {
            *self = Self::new();
        }

        // Stagger: the same spring, started at different moments.
        self.stagger_clock += input.dt;
        for (index, row) in self.rows.iter_mut().enumerate() {
            if !self.released[index] {
                if self.stagger_clock >= self.delays[index] {
                    self.released[index] = true;
                } else {
                    row.snap_to(1.0);
                    row.set_target(0.0);
                    continue;
                }
            }
            row.advance(input.dt);
        }

        self.pill.advance(input.dt);
        self.knob.advance(input.dt);
        self.sheet.advance(input.dt);
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        crate::caption(
            canvas,
            MARGIN,
            CONTENT_TOP + 4,
            "INTERFACE",
            "FOUR EVERYDAY ANIMATIONS, EACH ON A CONFIG PICKED FOR ITS JOB",
        );

        self.draw_tabs(canvas);
        self.draw_toggle(canvas);
        self.draw_list(canvas);
        self.draw_sheet(canvas);
    }

    fn draw_tabs(&self, canvas: &mut Canvas) {
        let y = tab_bar_y();
        canvas.rounded_rect(TAB_BAR_X, y, TAB_BAR_WIDTH, TAB_HEIGHT, 10.0, PANEL);

        let [x, width] = self.pill.value();
        canvas.rounded_rect(x + 3.0, y + 3.0, width - 6.0, TAB_HEIGHT - 6.0, 8.0, ACCENT);

        for (index, tab) in TABS.iter().enumerate() {
            let [slot_x, slot_width] = tab_slot(index);
            let text_x = slot_x + (slot_width - f64::from(crate::font::width(tab, 1))) / 2.0;
            let ink = if index == self.tab {
                BACKGROUND
            } else {
                TEXT_DIM
            };

            canvas.text(text_x.round() as i32, (y + 17.0) as i32, 1, ink, tab);
        }

        canvas.text(
            TAB_BAR_X as i32,
            (y - 16.0) as i32,
            1,
            TEXT,
            "TAB INDICATOR   BOUNCE 0.00   ONE SPRING FOR X AND WIDTH AT ONCE",
        );
    }

    fn draw_toggle(&self, canvas: &mut Canvas) {
        let y = toggle_y();
        let progress = self.knob.value();

        canvas.text(
            60,
            (y - 16.0) as i32,
            1,
            TEXT,
            "TOGGLE   BOUNCE 0.45   THE KNOB OVERSHOOTS, THE COLOUR FOLLOWS IT",
        );

        let track = canvas::mix(RAIL, GREEN, progress.clamp(0.0, 1.0));
        canvas.rounded_rect(60.0, y, 116.0, 34.0, 17.0, track);
        canvas.disc(60.0 + 17.0 + progress * 82.0, y + 17.0, 13.0, TEXT_BRIGHT);

        canvas.text(
            190,
            (y + 13.0) as i32,
            1,
            TEXT_DIM,
            if self.on { "ON" } else { "OFF" },
        );
    }

    fn draw_list(&self, canvas: &mut Canvas) {
        let top = list_top();
        canvas.text(
            60,
            (top - 16.0) as i32,
            1,
            TEXT,
            "STAGGERED LIST   BOUNCE 0.25   ONE SPRING PER ROW, STARTED 55MS APART",
        );

        for (index, row) in self.rows.iter().enumerate() {
            let offset = row.value();
            let y = top + index as f64 * 34.0;

            // Slide in from the right and fade up out of the background.
            let x = 60.0 + offset * 220.0;
            let fade = (1.0 - offset).clamp(0.0, 1.0);

            canvas.rounded_rect(x, y, 430.0, 26.0, 6.0, canvas::mix(BACKGROUND, PANEL, fade));
            canvas.rounded_rect(x, y, 4.0, 26.0, 2.0, canvas::mix(BACKGROUND, ACCENT, fade));
            canvas.text(
                (x + 16.0) as i32,
                (y + 10.0) as i32,
                1,
                canvas::mix(BACKGROUND, TEXT, fade),
                &format!(
                    "ROW {}   DELAY {:.0}MS",
                    index + 1,
                    self.delays[index] * 1000.0
                ),
            );
        }
    }

    fn draw_sheet(&self, canvas: &mut Canvas) {
        let progress = self.sheet.value();

        let width = 430.0;
        let height = 250.0;
        let x = f64::from(WIDTH - MARGIN) - width - 32.0;
        let hidden = f64::from(CONTENT_BOTTOM) - 8.0;
        let shown = f64::from(CONTENT_BOTTOM) - height - 20.0;
        let y = hidden + (shown - hidden) * progress;

        canvas.text(
            x as i32,
            CONTENT_TOP + 74,
            1,
            TEXT,
            "SHEET   BOUNCE 0.30   PRESS S",
        );
        canvas.text(
            x as i32,
            CONTENT_TOP + 90,
            1,
            TEXT_DIM,
            "IT CAN BE INTERRUPTED MID FLIGHT AND KEEPS ITS MOMENTUM",
        );

        canvas.rounded_rect(x, y, width, height, 14.0, PANEL);
        canvas.rounded_rect(x, y, width, 2.0, 1.0, PANEL_EDGE);
        canvas.rounded_rect(x + width / 2.0 - 24.0, y + 12.0, 48.0, 5.0, 2.5, RAIL);

        canvas.text(
            (x + 24.0) as i32,
            (y + 40.0) as i32,
            2,
            TEXT_BRIGHT,
            "SHEET",
        );
        canvas.text(
            (x + 24.0) as i32,
            (y + 64.0) as i32,
            1,
            TEXT_DIM,
            &format!(
                "PROGRESS {progress:.3}   VELOCITY {:+.2}",
                self.sheet.velocity()
            ),
        );
    }
}
