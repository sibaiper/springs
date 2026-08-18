# springs

Frame-rate-independent spring animations for Rust, powered by an analytical
damped-oscillator solver.

`springs` advances directly to the state at `t + dt`; it does not approximate
the motion with Euler integration or fixed substeps. A 20 FPS stutter and a
240 FPS display therefore follow the same trajectory when given the same
target changes at the same times.

The default build has no dependencies.

## Try the demos

Clone the repository and run any example:

_some of these are compute heavy, so make sure to run with --release_
```text
cargo run --example visual --release
cargo run --example matrix --release
cargo run --example gestures --release
cargo run --example gallery --release
cargo run --example shapes --release
cargo run --example bloom --release
```

- `visual` places per-frame advancement over a single analytical jump. The two
  markers should remain on top of each other.
- `matrix` makes duration and bounce tangible with 25 live step responses.
- `gestures` demonstrates drag, flick, momentum, snapping, and overscroll.
- `gallery` collects cursor following, shortest-path angles, interface motion,
  and phase portraits.
- `shapes` is a looping kinetic study: geometric forms morph, rotate, snap into
  new assemblies, and move in offset rhythms at the same time.
- `bloom` sends capsules, diamonds, and rings through five luminous spring
  choreographies with motion trails and interruptible transitions.

## API showcase

### A spring in one loop

```rust
use springs::{Spring, SpringConfig};

let config = SpringConfig::new()
    .duration(0.4)
    .bounce(0.2);

let mut spring = Spring::new(0.0)
    .with_target(100.0)
    .with_config(config);

// Call this with the real elapsed time for every rendered frame.
spring.advance(1.0 / 60.0);

println!("position: {}", spring.value());
println!("velocity: {}", spring.velocity());

// Retargeting preserves both position and momentum.
spring.set_target(40.0);
```

The `dt` passed to `advance` is measured in seconds. It is an elapsed-time
argument to the exact solution, not the step size of an approximate integrator.

### Anything you can subtract, you can spring

Numbers and arrays work without wrappers or adapters. With the optional `glam`
feature, its vector types do too.

```rust
use springs::Spring;

let opacity = Spring::new(0.0f32);
let point = Spring::new([0.0, 0.0]);
let colour = Spring::new([0.1, 0.4, 0.9]);
let transform = Spring::new([1.0; 16]);

# #[cfg(feature = "glam")]
# {
use glam::Vec3;
let position = Spring::new(Vec3::ZERO);
# }
```

Arrays can have any length. Their Euclidean magnitude determines when the
spring has settled, while every component follows the same exact transition.

### Angles take the short way round

```rust
use springs::{Angle, Spring};

let mut needle = Spring::new(Angle::from_degrees(359.0));
needle.set_target(Angle::from_degrees(2.0));

// Wrapped current-minus-target displacement: -3°.
// The needle travels forward 3°; a naive f64 spring travels backward 357°.
```

`Angle` stays normalized to `[0°, 360°)` and chooses the shortest signed
displacement whenever its target changes.

### Custom types: the trait is two methods

If an existing type can describe its displacement with a `SpringDelta`, making
it springable only requires implementing `SpringValue`. Arrays of floats already
implement `SpringDelta`, so an RGB colour needs no custom solver code:

```rust
use springs::{Spring, SpringValue};

#[derive(Clone, Copy)]
struct Rgb {
    red: f64,
    green: f64,
    blue: f64,
}

impl SpringValue for Rgb {
    type Delta = [f64; 3];

    fn displacement_from(self, target: Self) -> Self::Delta {
        [
            self.red - target.red,
            self.green - target.green,
            self.blue - target.blue,
        ]
    }

    fn add_displacement(self, [red, green, blue]: Self::Delta) -> Self {
        Self {
            red: self.red + red,
            green: self.green + green,
            blue: self.blue + blue,
        }
    }
}

let black = Rgb {
    red: 0.0,
    green: 0.0,
    blue: 0.0,
};
let white = Rgb {
    red: 1.0,
    green: 1.0,
    blue: 1.0,
};

let colour = Spring::new(black).with_target(white);
```

Keeping the animated channels continuous preserves the analytical solver's
frame-rate independence. Convert to `u8` only when displaying the current value,
rather than rounding the spring's state after every frame.

## Features

- Closed-form solutions for underdamped, critically damped, and overdamped
  springs
- Momentum-preserving retargeting
- `f32`, `f64`, arrays of any length, and shortest-path angles out of the box
- Optional support for `glam` vectors
- Custom value types through the `SpringValue` and `SpringDelta` traits
- Physical (`mass`, `stiffness`, `damping`) and animation-friendly (`duration`,
  `bounce`) configuration

## Installation

```toml
[dependencies]
springs = "0.1.2"
```

Enable the optional `glam` implementations with:

```toml
[dependencies]
springs = { version = "0.1.2", features = ["glam"] }
```

## Configuration

For animation-oriented controls, `duration` sets the spring's natural response
time and `bounce` selects the damping regime:

```rust
use springs::SpringConfig;

let config = SpringConfig::new()
    .duration(0.5)
    .bounce(0.25);
```

- `bounce > 0` is underdamped and overshoots.
- `bounce == 0` is critically damped and returns quickly without overshooting.
- `bounce < 0` is overdamped and approaches the target more gradually.

`duration` controls the natural frequency; it is not a hard deadline at which
the animation is forced to finish.

For conventional physical parameters:

```rust
use springs::SpringConfig;

let config = SpringConfig::physical()
    .mass(1.0)
    .stiffness(180.0)
    .damping(18.0)
    .build();
```

You can also configure the natural response and damping ratio directly:

```rust
use springs::SpringConfig;

let config = SpringConfig::responsive()
    .response(0.5)
    .damping(0.8)
    .build();
```

## Frame-rate independence and moving targets

For a fixed target, dividing the same elapsed time into any number of calls to
`advance` produces the same state, up to floating-point error. Retargeting also
preserves this property when the target changes are applied at the same physical
times.

A target supplied once per render frame is a sampled input. Different render
rates then supply different target histories, so no solver can make those inputs
identical without additional information about the target's motion. `springs`
models the target as constant between calls to `set_target`—a zero-order hold—and
solves each of those intervals exactly.

See [The mathematics](https://github.com/sibaiper/springs/blob/main/MATH.md) for
the equation of motion, the three closed-form transitions, and the precise
scope of the frame-rate-independence guarantee.

## License

Licensed under the
[MIT License](https://github.com/sibaiper/springs/blob/main/LICENSE).
