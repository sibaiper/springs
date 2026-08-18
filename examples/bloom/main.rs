//! A luminous choreography of spring-driven particles.
//!
//! Run it with:
//!
//! 
//! cargo run --example bloom --release
//!

#[path = "../common/canvas.rs"]
mod canvas;
#[path = "../common/font.rs"]
mod font;

use std::collections::VecDeque;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, TAU};
use std::time::Instant;

use canvas::Canvas;
use minifb::{Key, KeyRepeat, MouseButton, Window, WindowOptions};
use springs::{Spring, SpringConfig};

const WIDTH: i32 = 1280;
const HEIGHT: i32 = 820;
const MARGIN: i32 = 28;
const CENTER: (f64, f64) = (640.0, 400.0);
const PARTICLE_COUNT: usize = 24;
const TRAIL_SAMPLES: usize = 18;
const STAGE_DURATION: f64 = 1.8;

const BACKGROUND: u32 = 0x061017;
const BACKGROUND_RAISED: u32 = 0x0A1820;
const GRID: u32 = 0x102630;
const GRID_BRIGHT: u32 = 0x1D3C47;
const INK: u32 = 0xF3F4EC;
const INK_DIM: u32 = 0x79939B;
const INK_FAINT: u32 = 0x3D5962;
const MINT: u32 = 0x58F0CF;
const PINK: u32 = 0xFF4F91;
const GOLD: u32 = 0xFFC85A;
const VIOLET: u32 = 0x9A83FF;
const ICE: u32 = 0x86DFFF;
const PALETTE: [u32; 5] = [MINT, PINK, GOLD, VIOLET, ICE];

#[derive(Clone, Copy)]
struct Pose {
    position: [f64; 2],
    rotation: f64,
    length: f64,
    thickness: f64,
}

struct Particle {
    position: Spring<[f64; 2]>,
    rotation: Spring<f64>,
    length: Spring<f64>,
    thickness: Spring<f64>,
    color: Spring<[f64; 3]>,
    trail: VecDeque<[f64; 2]>,
}

impl Particle {
    fn new(index: usize, pose: Pose) -> Self {
        let position_config = SpringConfig::new()
            .duration(0.38 + (index % 5) as f64 * 0.025)
            .bounce(0.48 + (index % 4) as f64 * 0.08);

        Self {
            position: Spring::new(pose.position)
                .with_config(position_config)
                .with_epsilon(0.01),
            rotation: Spring::new(pose.rotation)
                .with_config(SpringConfig::new().duration(0.5).bounce(0.68))
                .with_epsilon(0.001),
            length: Spring::new(pose.length)
                .with_config(SpringConfig::new().duration(0.38).bounce(0.62))
                .with_epsilon(0.01),
            thickness: Spring::new(pose.thickness)
                .with_config(SpringConfig::new().duration(0.34).bounce(0.52))
                .with_epsilon(0.01),
            color: Spring::new(color_components(PALETTE[index % PALETTE.len()]))
                .with_config(SpringConfig::new().duration(0.55).bounce(0.0))
                .with_epsilon(0.01),
            trail: VecDeque::with_capacity(TRAIL_SAMPLES),
        }
    }

    fn retarget(&mut self, index: usize, stage_index: usize, transition_count: usize, pose: Pose) {
        self.position.set_target(pose.position);
        self.rotation
            .set_target(pose.rotation + f64::from(transition_count as u32) * TAU);
        self.length.set_target(pose.length);
        self.thickness.set_target(pose.thickness);
        self.color.set_target(color_components(
            PALETTE[(index + stage_index * 2) % PALETTE.len()],
        ));
    }

    fn update(&mut self, dt: f64) {
        self.position.advance(dt);
        self.rotation.advance(dt);
        self.length.advance(dt);
        self.thickness.advance(dt);
        self.color.advance(dt);

        if self.trail.len() == TRAIL_SAMPLES {
            self.trail.pop_front();
        }
        self.trail.push_back(self.position.value());
    }

    fn color(&self) -> u32 {
        components_color(self.color.value())
    }
}

struct Bloom {
    particles: Vec<Particle>,
    pulse: Spring<f64>,
    core_rotation: Spring<f64>,
    stage_index: usize,
    transition_count: usize,
    elapsed: f64,
    paused: bool,
    speed: f64,
}

impl Bloom {
    fn new() -> Self {
        let poses = stage_poses(0);
        let particles = poses
            .into_iter()
            .enumerate()
            .map(|(index, pose)| Particle::new(index, pose))
            .collect();

        Self {
            particles,
            pulse: Spring::new(1.0)
                .with_config(SpringConfig::new().duration(0.7).bounce(0.18))
                .with_epsilon(0.001),
            core_rotation: Spring::new(0.0)
                .with_config(SpringConfig::new().duration(0.62).bounce(0.56))
                .with_epsilon(0.001),
            stage_index: 0,
            transition_count: 0,
            elapsed: 0.0,
            paused: false,
            speed: 1.0,
        }
    }

    fn update(&mut self, dt: f64) {
        if self.paused {
            return;
        }

        let scaled_dt = dt * self.speed;
        self.elapsed += scaled_dt;
        while self.elapsed >= STAGE_DURATION {
            self.elapsed -= STAGE_DURATION;
            self.next_stage();
        }

        for particle in &mut self.particles {
            particle.update(scaled_dt);
        }
        self.pulse.advance(scaled_dt);
        self.core_rotation.advance(scaled_dt);
    }

    fn next_stage(&mut self) {
        self.stage_index = (self.stage_index + 1) % stage_names().len();
        self.transition_count += 1;
        let poses = stage_poses(self.stage_index);

        for (index, particle) in self.particles.iter_mut().enumerate() {
            particle.retarget(index, self.stage_index, self.transition_count, poses[index]);
        }

        self.pulse.snap_to(0.0);
        self.pulse.set_target(1.0);
        self.core_rotation
            .set_target(self.core_rotation.target() + FRAC_PI_2 + FRAC_PI_4);
    }

    fn trigger_next(&mut self) {
        self.elapsed = 0.0;
        self.next_stage();
    }

    fn progress(&self) -> f64 {
        (self.elapsed / STAGE_DURATION).clamp(0.0, 1.0)
    }

    fn stage_name(&self) -> &'static str {
        stage_names()[self.stage_index]
    }

    fn accent(&self) -> u32 {
        PALETTE[self.stage_index % PALETTE.len()]
    }
}

fn main() -> Result<(), minifb::Error> {
    let mut window = Window::new(
        "springs - kinetic bloom",
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions::default(),
    )?;
    window.set_target_fps(120);

    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    let mut bloom = Bloom::new();
    let mut clock = Instant::now();
    let mut mouse_was_down = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = clock.elapsed().as_secs_f64().min(0.1);
        clock = Instant::now();
        let mouse_is_down = window.get_mouse_down(MouseButton::Left);

        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            bloom.paused = !bloom.paused;
        }
        if window.is_key_pressed(Key::Enter, KeyRepeat::No) || (mouse_is_down && !mouse_was_down) {
            bloom.trigger_next();
        }
        if window.is_key_pressed(Key::Up, KeyRepeat::No) {
            bloom.speed = (bloom.speed + 0.25).min(2.5);
        }
        if window.is_key_pressed(Key::Down, KeyRepeat::No) {
            bloom.speed = (bloom.speed - 0.25).max(0.25);
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            bloom = Bloom::new();
        }

        mouse_was_down = mouse_is_down;
        bloom.update(dt);
        draw(&mut canvas, &bloom);
        window.update_with_buffer(canvas.buffer(), WIDTH as usize, HEIGHT as usize)?;
    }

    Ok(())
}

fn draw(canvas: &mut Canvas, bloom: &Bloom) {
    canvas.clear(BACKGROUND);
    draw_background(canvas);
    draw_connections(canvas, bloom);
    draw_trails(canvas, bloom);
    draw_particles(canvas, bloom);
    draw_core(canvas, bloom);
    draw_header(canvas, bloom);
    draw_stage_readout(canvas, bloom);
    draw_footer(canvas, bloom);
}

fn draw_background(canvas: &mut Canvas) {
    for x in (40..WIDTH).step_by(40) {
        for y in (80..HEIGHT - 64).step_by(40) {
            canvas.disc(f64::from(x), f64::from(y), 0.8, GRID);
        }
    }

    for radius in [96.0, 184.0, 278.0, 380.0] {
        canvas.ring(CENTER.0, CENTER.1, radius, 1.0, GRID);
    }

    canvas.line(
        (MARGIN.into(), CENTER.1),
        (CENTER.0 - 402.0, CENTER.1),
        1.0,
        GRID,
    );
    canvas.line(
        (CENTER.0 + 402.0, CENTER.1),
        (f64::from(WIDTH - MARGIN), CENTER.1),
        1.0,
        GRID,
    );
}

fn draw_connections(canvas: &mut Canvas, bloom: &Bloom) {
    for (index, particle) in bloom.particles.iter().enumerate() {
        let position = screen_position(particle.position.value());
        let next = screen_position(
            bloom.particles[(index + 1) % bloom.particles.len()]
                .position
                .value(),
        );
        let connection_color = canvas::mix(BACKGROUND, particle.color(), 0.2);

        canvas.line(position, next, 1.0, connection_color);
        if index % 3 == 0 {
            canvas.line(CENTER, position, 1.0, connection_color);
        }
    }
}

fn draw_trails(canvas: &mut Canvas, bloom: &Bloom) {
    for particle in &bloom.particles {
        let samples: Vec<_> = particle.trail.iter().copied().collect();

        for index in 1..samples.len() {
            let age = index as f64 / samples.len() as f64;
            let color = canvas::mix(BACKGROUND, particle.color(), age * 0.42);
            canvas.line(
                screen_position(samples[index - 1]),
                screen_position(samples[index]),
                1.0 + age * 2.0,
                color,
            );
        }
    }
}

fn draw_particles(canvas: &mut Canvas, bloom: &Bloom) {
    for (index, particle) in bloom.particles.iter().enumerate() {
        let center = screen_position(particle.position.value());
        match index % 3 {
            0 => draw_capsule(canvas, center, particle),
            1 => draw_diamond(canvas, center, particle),
            _ => draw_orb(canvas, center, particle),
        }
    }
}

fn draw_capsule(canvas: &mut Canvas, center: (f64, f64), particle: &Particle) {
    let direction = (
        particle.rotation.value().cos() * particle.length.value() * 0.5,
        particle.rotation.value().sin() * particle.length.value() * 0.5,
    );
    let from = (center.0 - direction.0, center.1 - direction.1);
    let to = (center.0 + direction.0, center.1 + direction.1);
    let thickness = particle.thickness.value().abs().max(3.0);

    canvas.line(from, to, thickness + 5.0, BACKGROUND);
    canvas.line(from, to, thickness, particle.color());
    canvas.disc(center.0, center.1, 2.2, INK);
}

fn draw_diamond(canvas: &mut Canvas, center: (f64, f64), particle: &Particle) {
    let length = particle.length.value().abs().max(12.0);
    let width = particle.thickness.value().abs().max(7.0) * 0.75;
    let rotation = particle.rotation.value();
    let points = transform_polygon(
        &[
            (-length * 0.5, 0.0),
            (0.0, -width),
            (length * 0.5, 0.0),
            (0.0, width),
        ],
        center,
        rotation,
    );

    canvas.polygon(&points, particle.color());
    canvas.polygon_outline(&points, 1.5, INK);
}

fn draw_orb(canvas: &mut Canvas, center: (f64, f64), particle: &Particle) {
    let radius = (particle.length.value().abs() * 0.18).clamp(7.0, 20.0);
    let weight = (particle.thickness.value().abs() * 0.24).clamp(2.0, 5.0);

    canvas.disc(center.0, center.1, radius + 3.0, BACKGROUND);
    canvas.ring(center.0, center.1, radius, weight, particle.color());
    let satellite = (
        center.0 + particle.rotation.value().cos() * radius,
        center.1 + particle.rotation.value().sin() * radius,
    );
    canvas.disc(satellite.0, satellite.1, 2.5, INK);
}

fn draw_core(canvas: &mut Canvas, bloom: &Bloom) {
    let pulse = bloom.pulse.value().clamp(0.0, 1.15);
    let pulse_color = canvas::mix(BACKGROUND, bloom.accent(), (1.0 - pulse).max(0.0) * 0.8);
    canvas.ring(CENTER.0, CENTER.1, 54.0 + pulse * 270.0, 2.0, pulse_color);

    let rotation = bloom.core_rotation.value();
    for index in 0..6 {
        let angle = rotation + index as f64 * TAU / 6.0;
        let inner = polar_point(CENTER, 28.0, angle);
        let outer = polar_point(CENTER, 58.0, angle);
        canvas.line(inner, outer, 5.0, bloom.accent());
    }

    canvas.disc(CENTER.0, CENTER.1, 32.0, BACKGROUND_RAISED);
    canvas.ring(CENTER.0, CENTER.1, 32.0, 2.0, INK);
    canvas.disc(CENTER.0, CENTER.1, 7.0 + pulse * 3.0, bloom.accent());
}

fn draw_header(canvas: &mut Canvas, bloom: &Bloom) {
    canvas.text(MARGIN, 24, 2, INK, "SPRINGS");
    canvas.text(
        MARGIN + 94,
        28,
        1,
        INK_DIM,
        "KINETIC BLOOM / GENERATIVE MOTION",
    );

    let status = if bloom.paused { "PAUSED" } else { "AUTOPLAY" };
    let status_width = font::width(status, 1) + 34;
    let status_x = WIDTH - MARGIN - status_width;
    canvas.rounded_rect(
        f64::from(status_x),
        20.0,
        f64::from(status_width),
        27.0,
        13.5,
        BACKGROUND_RAISED,
    );
    canvas.disc(
        f64::from(status_x + 14),
        33.5,
        3.0,
        if bloom.paused {
            INK_FAINT
        } else {
            bloom.accent()
        },
    );
    canvas.text(status_x + 25, 30, 1, INK, status);
    canvas.rect(MARGIN, 62, WIDTH - MARGIN * 2, 1, GRID_BRIGHT);
}

fn draw_stage_readout(canvas: &mut Canvas, bloom: &Bloom) {
    canvas.text(MARGIN, 668, 1, INK_DIM, "CURRENT COMPOSITION");
    canvas.text(MARGIN, 690, 4, INK, bloom.stage_name());

    let stage_number = format!("0{} / 05", bloom.stage_index + 1);
    canvas.text(
        WIDTH - MARGIN - font::width(&stage_number, 2),
        694,
        2,
        bloom.accent(),
        &stage_number,
    );
}

fn draw_footer(canvas: &mut Canvas, bloom: &Bloom) {
    let top = 758;
    canvas.rect(MARGIN, top, WIDTH - MARGIN * 2, 1, GRID_BRIGHT);

    let mut x = MARGIN;
    for (index, name) in stage_names().iter().enumerate() {
        let selected = index == bloom.stage_index;
        canvas.disc(
            f64::from(x + 4),
            f64::from(top + 23),
            if selected { 4.0 } else { 2.5 },
            if selected { bloom.accent() } else { INK_FAINT },
        );
        canvas.text(
            x + 15,
            top + 20,
            1,
            if selected { INK } else { INK_FAINT },
            name,
        );
        x += font::width(name, 1) + 42;
    }

    let controls = format!(
        "CLICK: NEXT  SPACE: PAUSE  UP/DOWN: SPEED  {:.2}X",
        bloom.speed
    );
    canvas.text(
        WIDTH - MARGIN - font::width(&controls, 1),
        top + 20,
        1,
        INK_DIM,
        &controls,
    );

    let progress_width = WIDTH - MARGIN * 2;
    canvas.rect(MARGIN, HEIGHT - 5, progress_width, 2, GRID);
    canvas.rect(
        MARGIN,
        HEIGHT - 5,
        (f64::from(progress_width) * bloom.progress()) as i32,
        2,
        bloom.accent(),
    );
}

fn stage_names() -> [&'static str; 5] {
    ["BLOOM", "APERTURE", "WAVE", "HELIX", "BURST"]
}

fn stage_poses(stage_index: usize) -> [Pose; PARTICLE_COUNT] {
    match stage_index {
        0 => bloom_poses(),
        1 => aperture_poses(),
        2 => wave_poses(),
        3 => helix_poses(),
        _ => burst_poses(),
    }
}

fn bloom_poses() -> [Pose; PARTICLE_COUNT] {
    std::array::from_fn(|index| {
        let angle = -FRAC_PI_2 + index as f64 * TAU / PARTICLE_COUNT as f64;
        let radius = if index % 2 == 0 { 176.0 } else { 272.0 };
        Pose {
            position: polar_offset(radius, angle),
            rotation: angle,
            length: if index % 2 == 0 { 78.0 } else { 48.0 },
            thickness: if index % 2 == 0 { 17.0 } else { 12.0 },
        }
    })
}

fn aperture_poses() -> [Pose; PARTICLE_COUNT] {
    std::array::from_fn(|index| {
        let ring = index / 8;
        let slot = index % 8;
        let angle = slot as f64 * TAU / 8.0 + ring as f64 * 0.28;
        let radius = 108.0 + ring as f64 * 88.0;
        Pose {
            position: polar_offset(radius, angle),
            rotation: angle + FRAC_PI_2,
            length: 70.0 - ring as f64 * 12.0,
            thickness: 15.0 - ring as f64 * 2.0,
        }
    })
}

fn wave_poses() -> [Pose; PARTICLE_COUNT] {
    std::array::from_fn(|index| {
        let progress = index as f64 / (PARTICLE_COUNT - 1) as f64;
        let x = -486.0 + progress * 972.0;
        let phase = progress * TAU * 1.65;
        let band = if index % 2 == 0 { -1.0 } else { 1.0 };
        Pose {
            position: [x, phase.sin() * 124.0 + band * 42.0],
            rotation: phase.cos().atan2(1.0),
            length: 82.0,
            thickness: 13.0,
        }
    })
}

fn helix_poses() -> [Pose; PARTICLE_COUNT] {
    std::array::from_fn(|index| {
        let progress = index as f64 / (PARTICLE_COUNT - 1) as f64;
        let phase = progress * TAU * 2.25;
        Pose {
            position: [phase.sin() * 252.0, -272.0 + progress * 544.0],
            rotation: phase + FRAC_PI_2,
            length: 44.0 + phase.cos().abs() * 50.0,
            thickness: 11.0 + phase.sin().abs() * 7.0,
        }
    })
}

fn burst_poses() -> [Pose; PARTICLE_COUNT] {
    const RADII: [f64; 6] = [122.0, 194.0, 282.0, 228.0, 316.0, 164.0];

    std::array::from_fn(|index| {
        let angle = -FRAC_PI_2 + index as f64 * TAU / PARTICLE_COUNT as f64;
        let radius = RADII[index % RADII.len()];
        Pose {
            position: polar_offset(radius, angle),
            rotation: angle,
            length: 34.0 + (index % 4) as f64 * 15.0,
            thickness: 10.0 + (index % 3) as f64 * 3.0,
        }
    })
}

fn polar_offset(radius: f64, angle: f64) -> [f64; 2] {
    [angle.cos() * radius, angle.sin() * radius]
}

fn polar_point(center: (f64, f64), radius: f64, angle: f64) -> (f64, f64) {
    (
        center.0 + angle.cos() * radius,
        center.1 + angle.sin() * radius,
    )
}

fn screen_position(position: [f64; 2]) -> (f64, f64) {
    (CENTER.0 + position[0], CENTER.1 + position[1])
}

fn transform_polygon(points: &[(f64, f64)], center: (f64, f64), rotation: f64) -> Vec<(f64, f64)> {
    let cosine = rotation.cos();
    let sine = rotation.sin();

    points
        .iter()
        .map(|(x, y)| {
            (
                center.0 + x * cosine - y * sine,
                center.1 + x * sine + y * cosine,
            )
        })
        .collect()
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
