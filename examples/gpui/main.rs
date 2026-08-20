//! A small GPUI dashboard whose motion is driven by `springs`.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example gpui --release
//! ```

use std::time::Instant;

use gpui::{
    App, Application, Bounds, Context, IntoElement, Render, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, relative, rgb, rgba, size,
};
use springs::{Spring, SpringConfig};

const WINDOW_WIDTH: f32 = 1040.0;
const WINDOW_HEIGHT: f32 = 720.0;
const TAB_WIDTH: f32 = 120.0;
const WORKFLOW_STEP_WIDTH: f32 = 176.0;
const WORKFLOW_GAP: f32 = 12.0;
const WORKFLOW_CURSOR_SIZE: f32 = 42.0;

const TABS: [&str; 3] = ["Overview", "Activity", "Settings"];
const WORKFLOW_STEPS: [(&str, &str); 3] = [("01", "Draft"), ("02", "Review"), ("03", "Ready")];

struct ReleaseDashboard {
    active_tab: usize,
    tab_indicator: Spring<f32>,
    workflow_step: usize,
    workflow_cursor: Spring<[f32; 2]>,
    auto_deploy_enabled: bool,
    auto_deploy_toggle: Spring<f32>,
    details_open: bool,
    details_panel: Spring<f32>,
    last_frame: Instant,
}

impl ReleaseDashboard {
    fn new() -> Self {
        Self {
            active_tab: 0,
            tab_indicator: Spring::new(0.0)
                .with_config(SpringConfig::new().duration(0.38).bounce(0.0))
                .with_epsilon(0.01),
            workflow_step: 1,
            workflow_cursor: Spring::new(workflow_cursor_position(0))
                .with_target(workflow_cursor_position(1))
                .with_config(SpringConfig::new().duration(0.55).bounce(0.32))
                .with_epsilon(0.05),
            auto_deploy_enabled: true,
            auto_deploy_toggle: Spring::new(1.0)
                .with_config(SpringConfig::new().duration(0.32).bounce(0.45))
                .with_epsilon(0.0005),
            details_open: true,
            details_panel: Spring::new(1.0)
                .with_config(SpringConfig::new().duration(0.46).bounce(0.18))
                .with_epsilon(0.0005),
            last_frame: Instant::now(),
        }
    }

    fn select_tab(&mut self, tab_index: usize, cx: &mut Context<Self>) {
        self.active_tab = tab_index;
        self.tab_indicator.set_target(tab_index as f32 * TAB_WIDTH);
        self.wake_animation(cx);
    }

    fn advance_workflow(&mut self, cx: &mut Context<Self>) {
        self.workflow_step = (self.workflow_step + 1) % WORKFLOW_STEPS.len();
        self.workflow_cursor
            .set_target(workflow_cursor_position(self.workflow_step));
        self.wake_animation(cx);
    }

    fn toggle_auto_deploy(&mut self, cx: &mut Context<Self>) {
        self.auto_deploy_enabled = !self.auto_deploy_enabled;
        self.auto_deploy_toggle
            .set_target(if self.auto_deploy_enabled { 1.0 } else { 0.0 });
        self.wake_animation(cx);
    }

    fn toggle_details(&mut self, cx: &mut Context<Self>) {
        self.details_open = !self.details_open;
        self.details_panel
            .set_target(if self.details_open { 1.0 } else { 0.0 });
        self.wake_animation(cx);
    }

    fn wake_animation(&mut self, cx: &mut Context<Self>) {
        self.last_frame = Instant::now();
        cx.notify();
    }

    fn advance_animation(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let elapsed_seconds = now.duration_since(self.last_frame).as_secs_f64().min(0.05);
        self.last_frame = now;

        self.tab_indicator.advance(elapsed_seconds);
        self.workflow_cursor.advance(elapsed_seconds);
        self.auto_deploy_toggle.advance(elapsed_seconds);
        self.details_panel.advance(elapsed_seconds);
        cx.notify();
    }

    fn is_animating(&self) -> bool {
        !self.tab_indicator.is_settled()
            || !self.workflow_cursor.is_settled()
            || !self.auto_deploy_toggle.is_settled()
            || !self.details_panel.is_settled()
    }

    fn schedule_animation_frame(&self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_animating() {
            return;
        }

        let dashboard = cx.entity().clone();
        window.on_next_frame(move |_, cx| {
            dashboard.update(cx, |dashboard, cx| dashboard.advance_animation(cx));
        });
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let indicator_left = self.tab_indicator.value();
        let mut tabs = div().relative().flex().h(px(48.0));

        for (tab_index, tab_label) in TABS.iter().enumerate() {
            let text_color = if tab_index == self.active_tab {
                rgb(0xf2f5f7)
            } else {
                rgb(0x727b87)
            };

            tabs = tabs.child(
                div()
                    .id(("tab", tab_index))
                    .w(px(TAB_WIDTH))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(text_color)
                    .cursor_pointer()
                    .hover(|tab| tab.bg(rgba(0xffffff08)))
                    .on_click(cx.listener(move |dashboard, _, _, cx| {
                        dashboard.select_tab(tab_index, cx);
                    }))
                    .child(*tab_label),
            );
        }

        div()
            .h(px(72.0))
            .px(px(28.0))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(0x242a31))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .size(px(34.0))
                            .rounded(px(10.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(0xe9ff60))
                            .text_color(rgb(0x111418))
                            .child("S"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(div().text_color(rgb(0xf2f5f7)).child("springs"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x727b87))
                                    .child("GPUI release control"),
                            ),
                    ),
            )
            .child(
                tabs.child(
                    div()
                        .absolute()
                        .bottom(px(0.0))
                        .left(px(indicator_left))
                        .w(px(TAB_WIDTH))
                        .h(px(2.0))
                        .px(px(22.0))
                        .child(div().size_full().rounded_full().bg(rgb(0xe9ff60))),
                ),
            )
            
    }

    fn render_workflow(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let [cursor_x, cursor_y] = self.workflow_cursor.value();
        let mut steps = div().relative().flex().gap(px(WORKFLOW_GAP)).pt(px(28.0));

        for (step_index, (_, step_name)) in WORKFLOW_STEPS.iter().enumerate() {
            let is_active = step_index == self.workflow_step;
            let step_color = if is_active {
                rgb(0xe9ff60)
            } else {
                rgb(0x343b44)
            };
            let label_color = if is_active {
                rgb(0xf2f5f7)
            } else {
                rgb(0x7c8590)
            };

            steps = steps.child(
                div()
                    .w(px(WORKFLOW_STEP_WIDTH))
                    .h(px(104.0))
                    .p(px(16.0))
                    .rounded(px(14.0))
                    .border_1()
                    .border_color(step_color)
                    .bg(rgb(0x191e24))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .child(div().text_color(label_color).child(*step_name)),
            );
        }

        div()
            .p(px(24.0))
            .rounded(px(18.0))
            .border_1()
            .border_color(rgb(0x292f37))
            .bg(rgb(0x151a20))
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(5.0))
                            .child(div().text_color(rgb(0xf2f5f7)).child("Release workflow"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x727b87))
                                    .child("Click repeatedly to retarget the spring mid-flight."),
                            ),
                    )
                    .child(
                        div()
                            .id("advance-workflow")
                            .px(px(14.0))
                            .py(px(8.0))
                            .rounded(px(9.0))
                            .bg(rgb(0xe9ff60))
                            .text_sm()
                            .text_color(rgb(0x111418))
                            .cursor_pointer()
                            .active(|button| button.opacity(0.75))
                            .on_click(cx.listener(|dashboard, _, _, cx| {
                                dashboard.advance_workflow(cx);
                            }))
                            .child("Retarget"),
                    ),
            )
            .child(
                steps.child(
                    div()
                        .absolute()
                        .left(px(cursor_x))
                        .top(px(cursor_y))
                        .size(px(WORKFLOW_CURSOR_SIZE))
                        .rounded_full()
                        .border_2()
                        .border_color(rgb(0x151a20))
                        .bg(rgb(0xe9ff60))
                        .shadow_lg()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(0x111418))
                        .child("→"),
                ),
            )
    }

    fn render_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let toggle_progress = self.auto_deploy_toggle.value();
        let toggle_left = 3.0 + toggle_progress * 22.0;
        let toggle_background = if self.auto_deploy_enabled {
            rgb(0xe9ff60)
        } else {
            rgb(0x343b44)
        };

        div()
            .flex()
            .gap(px(14.0))
            .child(
                div()
                    .flex_1()
                    .h(px(116.0))
                    .p(px(20.0))
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(rgb(0x292f37))
                    .bg(rgb(0x151a20))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(5.0))
                            .child(div().text_color(rgb(0xf2f5f7)).child("Auto deploy"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x727b87))
                                    .child("Bouncy scalar spring"),
                            ),
                    )
                    .child(
                        div()
                            .id("auto-deploy-toggle")
                            .relative()
                            .w(px(50.0))
                            .h(px(28.0))
                            .rounded_full()
                            .bg(toggle_background)
                            .cursor_pointer()
                            .on_click(cx.listener(|dashboard, _, _, cx| {
                                dashboard.toggle_auto_deploy(cx);
                            }))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(3.0))
                                    .left(px(toggle_left))
                                    .size(px(22.0))
                                    .rounded_full()
                                    .bg(rgb(0x111418)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h(px(116.0))
                    .p(px(20.0))
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(rgb(0x292f37))
                    .bg(rgb(0x151a20))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(5.0))
                            .child(div().text_color(rgb(0xf2f5f7)).child("Details"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x727b87))
                                    .child("Interruptible panel spring"),
                            ),
                    )
                    .child(
                        div()
                            .id("toggle-details")
                            .px(px(12.0))
                            .py(px(7.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(rgb(0x3a424c))
                            .text_sm()
                            .text_color(rgb(0xc7cdd4))
                            .cursor_pointer()
                            .hover(|button| button.bg(rgb(0x252b33)))
                            .on_click(cx.listener(|dashboard, _, _, cx| {
                                dashboard.toggle_details(cx);
                            }))
                            .child(if self.details_open { "Hide" } else { "Show" }),
                    ),
            )
    }

    fn render_details(&self) -> impl IntoElement {
        let panel_progress = self.details_panel.value();
        let panel_offset = (1.0 - panel_progress) * 34.0;
        let visible_progress = panel_progress.clamp(0.0, 1.0);

        div().relative().h(px(106.0)).overflow_hidden().child(
            div()
                .absolute()
                .top(px(panel_offset))
                .left(px(0.0))
                .right(px(0.0))
                .h(px(92.0))
                .px(px(20.0))
                .rounded(px(16.0))
                .border_1()
                .border_color(rgb(0x292f37))
                .bg(rgb(0x151a20))
                .opacity(visible_progress)
                .flex()
                .items_center()
                .justify_between()
                .child(status_detail("Frame loop", "GPUI on_next_frame"))
                .child(status_detail("Solver", "Analytical"))
                .child(status_detail(
                    "Current value",
                    &format!("{panel_progress:.3}"),
                ))
                .child(
                    div()
                        .w(px(210.0))
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x727b87))
                                .child("SETTLE PROGRESS"),
                        )
                        .child(
                            div()
                                .h(px(5.0))
                                .w_full()
                                .rounded_full()
                                .bg(rgb(0x252b33))
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(visible_progress))
                                        .rounded_full()
                                        .bg(rgb(0xe9ff60)),
                                ),
                        ),
                ),
        )
    }
}

impl Render for ReleaseDashboard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.schedule_animation_frame(window, cx);

        div()
            .size_full()
            .bg(rgb(0x101419))
            .text_color(rgb(0xf2f5f7))
            .child(self.render_header(cx))
            .child(
                div()
                    .px(px(42.0))
                    .py(px(30.0))
                    .flex()
                    .flex_col()
                    .gap(px(18.0))
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_2xl()
                                            .text_color(rgb(0xf2f5f7))
                                            .child("GPUI interface animated by springs"),
                                    )
                            )
                    )
                    .child(self.render_workflow(cx))
                    .child(self.render_controls(cx))
                    .child(self.render_details()),
            )
    }
}

fn workflow_cursor_position(step_index: usize) -> [f32; 2] {
    let step_offset = step_index as f32 * (WORKFLOW_STEP_WIDTH + WORKFLOW_GAP);
    let horizontal_center = (WORKFLOW_STEP_WIDTH - WORKFLOW_CURSOR_SIZE) / 2.0;
    [step_offset + horizontal_center, 6.0]
}

fn status_detail(label: &str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x727b87))
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0xd9dee4))
                .child(value.to_owned()),
        )
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| ReleaseDashboard::new()),
        )
        .expect("GPUI should open the springs example window");
        cx.activate(true);
    });
}
