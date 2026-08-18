//! A kinetic study of spring-driven shape morphing.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example shapes --release
//! ```

#[path = "../common/canvas.rs"]
mod canvas;
#[path = "../common/font.rs"]
mod font;

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};
use std::time::Instant;

use canvas::Canvas;
use minifb::{Key, KeyRepeat, MouseButton, Window, WindowOptions};
use springs::{Spring, SpringConfig};

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 820;
const MARGIN: i32 = 24;
const HEADER_HEIGHT: i32 = 76;
const FOOTER_TOP: i32 = 770;

const HERO_X: i32 = 24;
const HERO_Y: i32 = 88;
const HERO_WIDTH: i32 = 772;
const HERO_HEIGHT: i32 = 658;
const SIDE_X: i32 = 812;
const SIDE_WIDTH: i32 = 444;
const SIDE_TOP_HEIGHT: i32 = 318;
const SIDE_BOTTOM_Y: i32 = 422;
const SIDE_BOTTOM_HEIGHT: i32 = 324;

const BACKGROUND: u32 = 0x090A0A;
const PANEL: u32 = 0x111312;
const PANEL_RAISED: u32 = 0x171A18;
const GRID: u32 = 0x202420;
const GRID_BRIGHT: u32 = 0x343A34;
const INK: u32 = 0xF3F2E9;
const INK_DIM: u32 = 0x8D958B;
const INK_FAINT: u32 = 0x565D56;
const ACID: u32 = 0xDDFE52;
const CORAL: u32 = 0xFF6B4A;
const CYAN: u32 = 0x72E3F4;
const VIOLET: u32 = 0xAE8BFF;
const YELLOW: u32 = 0xFFC857;

const POINT_COUNT: usize = 64;
const COMPONENT_COUNT: usize = POINT_COUNT * 2;
const FORM_INTERVAL: f64 = 1.75;
const FORMS: [Form; 6] = [
    Form::Circle,
    Form::Rectangle,
    Form::Triangle,
    Form::Star,
    Form::Hexagon,
    Form::Diamond,
];
const FORM_COLORS: [u32; 6] = [ACID, CYAN, CORAL, VIOLET, YELLOW, ACID];

#[derive(Clone, Copy)]
enum Form {
    Circle,
    Rectangle,
    Triangle,
    Star,
    Hexagon,
    Diamond,
}

impl Form {
    fn name(self) -> &'static str {
        match self {
            Self::Circle => "CIRCLE",
            Self::Rectangle => "RECTANGLE",
            Self::Triangle => "TRIANGLE",
            Self::Star => "STAR",
            Self::Hexagon => "HEXAGON",
            Self::Diamond => "DIAMOND",
        }
    }

    fn vertices(self) -> Vec<(f64, f64)> {
        match self {
            Self::Circle => Vec::new(),
            Self::Rectangle => vec![(-1.0, -0.62), (1.0, -0.62), (1.0, 0.62), (-1.0, 0.62)],
            Self::Triangle => regular_polygon(3, -FRAC_PI_2),
            Self::Star => star_polygon(5, -FRAC_PI_2, 0.43),
            Self::Hexagon => regular_polygon(6, 0.0),
            Self::Diamond => vec![(0.0, -1.0), (0.78, 0.0), (0.0, 1.0), (-0.78, 0.0)],
        }
    }
}

struct MorphingShape {
    points: Spring<[f64; COMPONENT_COUNT]>,
    rotation: Spring<f64>,
    scale: Spring<f64>,
    color: Spring<[f64; 3]>,
    form_index: usize,
    elapsed: f64,
    interval: f64,
}

impl MorphingShape {
    fn new(form_index: usize, elapsed: f64, interval: f64, bounce: f64) -> Self {
        let form_index = form_index % FORMS.len();
        let points = shape_components(FORMS[form_index]);
        let color = color_components(FORM_COLORS[form_index]);

        Self {
            points: Spring::new(points)
                .with_config(SpringConfig::new().duration(0.72).bounce(bounce))
                .with_epsilon(0.0005),
            rotation: Spring::new(0.0)
                .with_config(SpringConfig::new().duration(0.62).bounce(0.58))
                .with_epsilon(0.0005),
            scale: Spring::new(1.0)
                .with_config(SpringConfig::new().duration(0.48).bounce(0.7))
                .with_epsilon(0.0005),
            color: Spring::new(color)
                .with_config(SpringConfig::new().duration(0.6).bounce(0.0))
                .with_epsilon(0.0005),
            form_index,
            elapsed,
            interval,
        }
    }

    fn update(&mut self, dt: f64) -> bool {
        self.elapsed += dt;
        let mut transitioned = false;

        while self.elapsed >= self.interval {
            self.elapsed -= self.interval;
            self.next();
            transitioned = true;
        }

        self.points.advance(dt);
        self.rotation.advance(dt);
        self.scale.advance(dt);
        self.color.advance(dt);
        transitioned
    }

    fn next(&mut self) {
        self.form_index = (self.form_index + 1) % FORMS.len();
        self.points
            .set_target(shape_components(FORMS[self.form_index]));
        self.rotation
            .set_target(self.rotation.target() + rotation_step(self.form_index));
        self.scale
            .set_target(if self.form_index % 2 == 0 { 1.0 } else { 0.92 });
        self.color
            .set_target(color_components(FORM_COLORS[self.form_index]));
    }

    fn current_form(&self) -> Form {
        FORMS[self.form_index]
    }

    fn next_form(&self) -> Form {
        FORMS[(self.form_index + 1) % FORMS.len()]
    }

    fn progress(&self) -> f64 {
        (self.elapsed / self.interval).clamp(0.0, 1.0)
    }

    fn transformed_points(&self, center: (f64, f64), radius: f64) -> Vec<(f64, f64)> {
        transform_components(
            self.points.value(),
            center,
            radius * self.scale.value(),
            self.rotation.value(),
        )
    }

    fn target_points(&self, center: (f64, f64), radius: f64) -> Vec<(f64, f64)> {
        transform_components(
            self.points.target(),
            center,
            radius * self.scale.target(),
            self.rotation.target(),
        )
    }

    fn color(&self) -> u32 {
        components_color(self.color.value())
    }
}

struct AssemblyPiece {
    position: Spring<[f64; 2]>,
    rotation: Spring<f64>,
}

struct Assembly {
    pieces: Vec<AssemblyPiece>,
    layout_index: usize,
}

impl Assembly {
    fn new() -> Self {
        let targets = assembly_targets(0);
        let pieces = targets
            .into_iter()
            .enumerate()
            .map(|(index, position)| AssemblyPiece {
                position: Spring::new(position)
                    .with_config(
                        SpringConfig::new()
                            .duration(0.46 + index as f64 * 0.018)
                            .bounce(0.74),
                    )
                    .with_epsilon(0.01),
                rotation: Spring::new(0.0)
                    .with_config(SpringConfig::new().duration(0.52).bounce(0.68))
                    .with_epsilon(0.001),
            })
            .collect();

        Self {
            pieces,
            layout_index: 0,
        }
    }

    fn next(&mut self) {
        self.layout_index = (self.layout_index + 1) % 4;
        let targets = assembly_targets(self.layout_index);

        for (index, piece) in self.pieces.iter_mut().enumerate() {
            piece.position.set_target(targets[index]);
            piece
                .rotation
                .set_target(piece.rotation.target() + FRAC_PI_4 * (1.0 + (index % 3) as f64));
        }
    }

    fn update(&mut self, dt: f64) {
        for piece in &mut self.pieces {
            piece.position.advance(dt);
            piece.rotation.advance(dt);
        }
    }
}

struct Demo {
    hero: MorphingShape,
    miniatures: [MorphingShape; 3],
    assembly: Assembly,
    paused: bool,
    speed: f64,
}

impl Demo {
    fn new() -> Self {
        Self {
            hero: MorphingShape::new(0, 0.0, FORM_INTERVAL, 0.42),
            miniatures: [
                MorphingShape::new(2, 0.15, 1.35, 0.58),
                MorphingShape::new(4, 0.72, 1.58, 0.36),
                MorphingShape::new(1, 1.08, 1.92, 0.72),
            ],
            assembly: Assembly::new(),
            paused: false,
            speed: 1.0,
        }
    }

    fn update(&mut self, dt: f64) {
        if self.paused {
            return;
        }

        let scaled_dt = dt * self.speed;
        if self.hero.update(scaled_dt) {
            self.assembly.next();
        }
        for miniature in &mut self.miniatures {
            miniature.update(scaled_dt);
        }
        self.assembly.update(scaled_dt);
    }

    fn next(&mut self) {
        self.hero.next();
        self.hero.elapsed = 0.0;
        self.assembly.next();
        for miniature in &mut self.miniatures {
            miniature.next();
        }
    }
}

fn main() -> Result<(), minifb::Error> {
    let mut window = Window::new(
        "springs - kinetic shape study",
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(120);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut demo = Demo::new();
    let mut clock = Instant::now();
    let mut mouse_was_down = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = clock.elapsed().as_secs_f64().min(0.1);
        clock = Instant::now();
        let mouse_is_down = window.get_mouse_down(MouseButton::Left);

        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            demo.paused = !demo.paused;
        }
        if window.is_key_pressed(Key::Enter, KeyRepeat::No) || (mouse_is_down && !mouse_was_down) {
            demo.next();
        }
        if window.is_key_pressed(Key::Up, KeyRepeat::No) {
            demo.speed = (demo.speed + 0.25).min(2.5);
        }
        if window.is_key_pressed(Key::Down, KeyRepeat::No) {
            demo.speed = (demo.speed - 0.25).max(0.25);
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            demo = Demo::new();
        }

        mouse_was_down = mouse_is_down;
        demo.update(dt);
        draw(&mut canvas, &demo);
        window.update_with_buffer(canvas.buffer(), WIDTH as usize, HEIGHT as usize)?;
    }

    Ok(())
}

fn draw(canvas: &mut Canvas, demo: &Demo) {
    canvas.clear(BACKGROUND);
    draw_background_grid(canvas);
    draw_header(canvas, demo);
    draw_hero(canvas, &demo.hero);
    draw_assembly(canvas, &demo.assembly);
    draw_miniatures(canvas, &demo.miniatures);
    draw_footer(canvas, demo);
}

fn draw_background_grid(canvas: &mut Canvas) {
    for x in (MARGIN..WIDTH - MARGIN).step_by(48) {
        canvas.rect(x, HEADER_HEIGHT, 1, FOOTER_TOP - HEADER_HEIGHT, 0x101210);
    }
    for y in (HEADER_HEIGHT..FOOTER_TOP).step_by(48) {
        canvas.rect(MARGIN, y, WIDTH - MARGIN * 2, 1, 0x101210);
    }
}

fn draw_header(canvas: &mut Canvas, demo: &Demo) {
    canvas.text(MARGIN, 24, 3, INK, "SPRINGS");
    canvas.text(MARGIN + 140, 28, 1, INK_DIM, "KINETIC SHAPE STUDY / 05");

    let status = if demo.paused { "PAUSED" } else { "LIVE" };
    let status_width = font::width(status, 1) + 34;
    let status_x = WIDTH - MARGIN - status_width;
    canvas.rounded_rect(
        f64::from(status_x),
        20.0,
        f64::from(status_width),
        28.0,
        14.0,
        PANEL_RAISED,
    );
    canvas.disc(
        f64::from(status_x + 14),
        34.0,
        3.5,
        if demo.paused { INK_FAINT } else { ACID },
    );
    canvas.text(status_x + 25, 31, 1, INK, status);
    canvas.rect(MARGIN, 64, WIDTH - MARGIN * 2, 1, GRID_BRIGHT);
}

fn draw_hero(canvas: &mut Canvas, shape: &MorphingShape) {
    draw_panel(canvas, HERO_X, HERO_Y, HERO_WIDTH, HERO_HEIGHT);
    draw_panel_label(
        canvas,
        HERO_X,
        HERO_Y,
        "01",
        "FORM / CONTINUOUS TRANSFORMATION",
    );

    let center = (f64::from(HERO_X + HERO_WIDTH / 2), 390.0);
    let target = shape.target_points(center, 214.0);
    let points = shape.transformed_points(center, 214.0);
    let shadow: Vec<_> = points.iter().map(|(x, y)| (x + 14.0, y + 16.0)).collect();

    draw_crosshair(canvas, center, 275.0);
    canvas.polygon(&shadow, 0x070807);
    canvas.polygon_outline(&target, 1.0, GRID_BRIGHT);
    canvas.polygon(&points, shape.color());
    canvas.polygon_outline(&points, 2.0, INK);
    draw_shape_core(canvas, center, shape.rotation.value());

    let label_y = HERO_Y + HERO_HEIGHT - 92;
    canvas.text(HERO_X + 28, label_y, 1, INK_DIM, "NOW");
    canvas.text(
        HERO_X + 28,
        label_y + 20,
        3,
        INK,
        shape.current_form().name(),
    );

    let next_label = format!("NEXT / {}", shape.next_form().name());
    let next_x = HERO_X + HERO_WIDTH - 28 - font::width(&next_label, 1);
    canvas.text(next_x, label_y + 28, 1, INK_DIM, &next_label);

    let progress_x = HERO_X + 28;
    let progress_y = HERO_Y + HERO_HEIGHT - 26;
    let progress_width = HERO_WIDTH - 56;
    canvas.rounded_rect(
        f64::from(progress_x),
        f64::from(progress_y),
        f64::from(progress_width),
        4.0,
        2.0,
        GRID_BRIGHT,
    );
    canvas.rounded_rect(
        f64::from(progress_x),
        f64::from(progress_y),
        f64::from(progress_width) * shape.progress(),
        4.0,
        2.0,
        shape.color(),
    );
}

fn draw_crosshair(canvas: &mut Canvas, center: (f64, f64), radius: f64) {
    canvas.line(
        (center.0 - radius, center.1),
        (center.0 + radius, center.1),
        1.0,
        GRID,
    );
    canvas.line(
        (center.0, center.1 - radius),
        (center.0, center.1 + radius),
        1.0,
        GRID,
    );
    canvas.ring(center.0, center.1, radius, 1.0, GRID);
    canvas.ring(center.0, center.1, radius * 0.52, 1.0, GRID);
}

fn draw_shape_core(canvas: &mut Canvas, center: (f64, f64), rotation: f64) {
    canvas.disc(center.0, center.1, 33.0, BACKGROUND);
    canvas.ring(center.0, center.1, 33.0, 2.0, INK);
    let direction = (rotation.cos() * 20.0, rotation.sin() * 20.0);
    canvas.line(
        (center.0 - direction.0, center.1 - direction.1),
        (center.0 + direction.0, center.1 + direction.1),
        3.0,
        INK,
    );
}

fn draw_assembly(canvas: &mut Canvas, assembly: &Assembly) {
    draw_panel(canvas, SIDE_X, HERO_Y, SIDE_WIDTH, SIDE_TOP_HEIGHT);
    draw_panel_label(canvas, SIDE_X, HERO_Y, "02", "SNAP / NINE BODY ASSEMBLY");

    let card_origin = [f64::from(SIDE_X + 32), f64::from(HERO_Y + 70)];
    let colors = [ACID, CYAN, CORAL, VIOLET, YELLOW, CYAN, CORAL, ACID, VIOLET];

    for (index, piece) in assembly.pieces.iter().enumerate() {
        let [x, y] = piece.position.value();
        let center = (card_origin[0] + x, card_origin[1] + y);
        let size = if index == 4 { 28.0 } else { 21.0 };
        let points = rotated_square(center, size, piece.rotation.value());
        canvas.polygon(&points, colors[index]);
        canvas.polygon_outline(&points, 1.5, INK);
    }

    let layout_name = ["GRID", "ORBIT", "WAVE", "CROSS"][assembly.layout_index];
    canvas.text(
        SIDE_X + 28,
        HERO_Y + SIDE_TOP_HEIGHT - 30,
        1,
        INK_DIM,
        "LAYOUT",
    );
    canvas.text(
        SIDE_X + 90,
        HERO_Y + SIDE_TOP_HEIGHT - 30,
        1,
        INK,
        layout_name,
    );
    canvas.text(
        SIDE_X + SIDE_WIDTH - 164,
        HERO_Y + SIDE_TOP_HEIGHT - 30,
        1,
        INK_FAINT,
        "BOUNCE 0.74",
    );
}

fn draw_miniatures(canvas: &mut Canvas, shapes: &[MorphingShape; 3]) {
    draw_panel(
        canvas,
        SIDE_X,
        SIDE_BOTTOM_Y,
        SIDE_WIDTH,
        SIDE_BOTTOM_HEIGHT,
    );
    draw_panel_label(
        canvas,
        SIDE_X,
        SIDE_BOTTOM_Y,
        "03",
        "CHORUS / OFFSET OSCILLATORS",
    );

    let centers = [
        (f64::from(SIDE_X + 84), f64::from(SIDE_BOTTOM_Y + 170)),
        (f64::from(SIDE_X + 222), f64::from(SIDE_BOTTOM_Y + 170)),
        (f64::from(SIDE_X + 360), f64::from(SIDE_BOTTOM_Y + 170)),
    ];

    for (index, shape) in shapes.iter().enumerate() {
        let points = shape.transformed_points(centers[index], 52.0);
        let shadow: Vec<_> = points.iter().map(|(x, y)| (x + 5.0, y + 6.0)).collect();
        canvas.polygon(&shadow, BACKGROUND);
        canvas.polygon(&points, shape.color());
        canvas.polygon_outline(&points, 1.25, INK);
        canvas.text(
            centers[index].0 as i32 - font::width(shape.current_form().name(), 1) / 2,
            SIDE_BOTTOM_Y + 245,
            1,
            INK_DIM,
            shape.current_form().name(),
        );
    }

    canvas.text(
        SIDE_X + 28,
        SIDE_BOTTOM_Y + SIDE_BOTTOM_HEIGHT - 28,
        1,
        INK_FAINT,
        "SAME SOLVER / DIFFERENT PHASE / ZERO KEYFRAMES",
    );
}

fn draw_footer(canvas: &mut Canvas, demo: &Demo) {
    canvas.rect(MARGIN, FOOTER_TOP, WIDTH - MARGIN * 2, 1, GRID_BRIGHT);
    canvas.text(
        MARGIN,
        FOOTER_TOP + 21,
        1,
        INK_DIM,
        "CLICK OR ENTER: NEXT    SPACE: PAUSE    UP/DOWN: SPEED    R: RESET    ESC: QUIT",
    );
    let speed = format!("{:.2}X", demo.speed);
    canvas.text(
        WIDTH - MARGIN - font::width(&speed, 2),
        FOOTER_TOP + 16,
        2,
        if demo.paused { INK_FAINT } else { ACID },
        &speed,
    );
}

fn draw_panel(canvas: &mut Canvas, x: i32, y: i32, width: i32, height: i32) {
    canvas.rounded_rect(
        f64::from(x),
        f64::from(y),
        f64::from(width),
        f64::from(height),
        12.0,
        GRID,
    );
    canvas.rounded_rect(
        f64::from(x + 1),
        f64::from(y + 1),
        f64::from(width - 2),
        f64::from(height - 2),
        11.0,
        PANEL,
    );
}

fn draw_panel_label(canvas: &mut Canvas, x: i32, y: i32, index: &str, label: &str) {
    canvas.rounded_rect(f64::from(x + 20), f64::from(y + 18), 28.0, 22.0, 5.0, ACID);
    canvas.text(x + 28, y + 26, 1, BACKGROUND, index);
    canvas.text(x + 60, y + 26, 1, INK_DIM, label);
}

fn regular_polygon(sides: usize, rotation: f64) -> Vec<(f64, f64)> {
    (0..sides)
        .map(|index| {
            let angle = rotation + index as f64 * TAU / sides as f64;
            (angle.cos(), angle.sin())
        })
        .collect()
}

fn star_polygon(points: usize, rotation: f64, inner_radius: f64) -> Vec<(f64, f64)> {
    (0..points * 2)
        .map(|index| {
            let radius = if index % 2 == 0 { 1.0 } else { inner_radius };
            let angle = rotation + index as f64 * PI / points as f64;
            (angle.cos() * radius, angle.sin() * radius)
        })
        .collect()
}

fn shape_components(form: Form) -> [f64; COMPONENT_COUNT] {
    let vertices = form.vertices();
    let mut components = [0.0; COMPONENT_COUNT];

    for point_index in 0..POINT_COUNT {
        let angle = -FRAC_PI_2 + point_index as f64 * TAU / POINT_COUNT as f64;
        let radius = if vertices.is_empty() {
            1.0
        } else {
            radial_intersection(&vertices, angle)
        };
        components[point_index * 2] = angle.cos() * radius;
        components[point_index * 2 + 1] = angle.sin() * radius;
    }

    components
}

fn radial_intersection(vertices: &[(f64, f64)], angle: f64) -> f64 {
    let direction = (angle.cos(), angle.sin());
    let mut nearest = f64::INFINITY;

    for index in 0..vertices.len() {
        let from = vertices[index];
        let to = vertices[(index + 1) % vertices.len()];
        let edge = (to.0 - from.0, to.1 - from.1);
        let denominator = cross(direction, edge);

        if denominator.abs() < 1.0e-9 {
            continue;
        }

        let distance = cross(from, edge) / denominator;
        let along_edge = cross(from, direction) / denominator;
        if distance >= 0.0 && (0.0..=1.0).contains(&along_edge) {
            nearest = nearest.min(distance);
        }
    }

    if nearest.is_finite() { nearest } else { 1.0 }
}

fn cross(left: (f64, f64), right: (f64, f64)) -> f64 {
    left.0 * right.1 - left.1 * right.0
}

fn transform_components(
    components: [f64; COMPONENT_COUNT],
    center: (f64, f64),
    radius: f64,
    rotation: f64,
) -> Vec<(f64, f64)> {
    let cosine = rotation.cos();
    let sine = rotation.sin();

    (0..POINT_COUNT)
        .map(|index| {
            let x = components[index * 2] * radius;
            let y = components[index * 2 + 1] * radius;
            (
                center.0 + x * cosine - y * sine,
                center.1 + x * sine + y * cosine,
            )
        })
        .collect()
}

fn rotated_square(center: (f64, f64), size: f64, rotation: f64) -> Vec<(f64, f64)> {
    let cosine = rotation.cos();
    let sine = rotation.sin();

    [(-size, -size), (size, -size), (size, size), (-size, size)]
        .into_iter()
        .map(|(x, y)| {
            (
                center.0 + x * cosine - y * sine,
                center.1 + x * sine + y * cosine,
            )
        })
        .collect()
}

fn rotation_step(index: usize) -> f64 {
    [
        FRAC_PI_4,
        PI / 3.0,
        -FRAC_PI_4,
        PI / 2.5,
        -PI / 3.0,
        FRAC_PI_2,
    ][index]
}

fn color_components(color: u32) -> [f64; 3] {
    [
        f64::from((color >> 16) & 0xFF),
        f64::from((color >> 8) & 0xFF),
        f64::from(color & 0xFF),
    ]
}

fn components_color(components: [f64; 3]) -> u32 {
    let channel = |value: f64| value.round().clamp(0.0, 255.0) as u32;
    (channel(components[0]) << 16) | (channel(components[1]) << 8) | channel(components[2])
}

fn assembly_targets(layout_index: usize) -> [[f64; 2]; 9] {
    match layout_index {
        0 => std::array::from_fn(|index| {
            [
                92.0 + (index % 3) as f64 * 108.0,
                32.0 + (index / 3) as f64 * 62.0,
            ]
        }),
        1 => std::array::from_fn(|index| {
            let angle = -FRAC_PI_2 + index as f64 * TAU / 9.0;
            [188.0 + angle.cos() * 124.0, 104.0 + angle.sin() * 70.0]
        }),
        2 => std::array::from_fn(|index| {
            let x = 38.0 + index as f64 * 38.0;
            [x, 104.0 + (index as f64 * 0.9).sin() * 60.0]
        }),
        _ => [
            [188.0, 20.0],
            [188.0, 62.0],
            [188.0, 104.0],
            [188.0, 146.0],
            [188.0, 168.0],
            [100.0, 104.0],
            [144.0, 104.0],
            [232.0, 104.0],
            [276.0, 104.0],
        ],
    }
}
