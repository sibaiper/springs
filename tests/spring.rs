//! Behavioural tests for the spring solver.
//!
//! Everything here drives the public API only: a [`Spring`] is stepped with
//! `advance` and its observable value/velocity are checked against either the
//! equation of motion itself or a property the animation must have (no
//! overshoot, finiteness, frame-rate independence, ...).

use springs::{Angle, Spring, SpringConfig, SpringDelta, SpringValue};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Response shared by every fixture (ω₀ = 2π / 0.6 ≈ 10.47 rad/s), chosen so
/// the springs are still visibly in motion across the horizons sampled below.
const RESPONSE: f64 = 0.6;

/// ζ = 0.3 — oscillatory, takes the underdamped branch.
fn underdamped() -> SpringConfig {
    SpringConfig::from_response_damping(RESPONSE, 0.3)
}

/// ζ = 1 — the boundary case, which has a branch of its own.
fn critically_damped() -> SpringConfig {
    SpringConfig::from_response_damping(RESPONSE, 1.0)
}

/// ζ = 2 — two distinct real roots, takes the overdamped branch.
fn overdamped() -> SpringConfig {
    SpringConfig::from_response_damping(RESPONSE, 2.0)
}

/// The three solver branches, for tests that must hold in every regime.
fn regimes() -> [(&'static str, SpringConfig); 3] {
    [
        ("underdamped", underdamped()),
        ("critically damped", critically_damped()),
        ("overdamped", overdamped()),
    ]
}

/// A unit step: sitting at 0, asked to travel to 1, at rest.
fn step(config: SpringConfig) -> Spring<f64> {
    Spring::new(0.0).with_target(1.0).with_config(config)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[track_caller]
fn assert_close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    let error = (actual - expected).abs();
    assert!(
        error <= tolerance,
        "{what}: expected {expected}, got {actual} (off by {error:e}, tolerance {tolerance:e})"
    );
}

/// Steps `spring` forward by `seconds`, one `1.0 / rate` frame at a time.
fn advance_for(spring: &mut Spring<f64>, seconds: f64, rate: f64) {
    let dt = 1.0 / rate;
    for _ in 0..(seconds * rate).round() as u32 {
        spring.advance(dt);
    }
}

/// Samples a 0 → 1 step response at 1 kHz for five seconds.
fn step_response(config: SpringConfig) -> Vec<f64> {
    let mut spring = step(config);
    (0..5_000)
        .map(|_| {
            spring.advance(1.0 / 1_000.0);
            spring.value()
        })
        .collect()
}

fn peak(samples: &[f64]) -> f64 {
    samples.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// True if the trajectory never moves backwards (modulo float noise).
fn rises_monotonically(samples: &[f64]) -> bool {
    samples.windows(2).all(|pair| pair[1] >= pair[0] - 1e-12)
}

// ---------------------------------------------------------------------------
// The analytical solution
// ---------------------------------------------------------------------------

/// The solver integrates the ODE in closed form rather than stepping it, so the
/// state after `t` seconds must not depend on how `t` was chopped into frames.
/// A 20 fps stutter and a 240 fps display have to land on the same pixel.
#[test]
fn analytical_solution_is_frame_rate_independent() {
    const RATES: [f64; 5] = [20.0, 40.0, 60.0, 120.0, 240.0];
    const INTERVAL: f64 = 0.05;
    const CHECKPOINTS: u32 = 6;
    const TOLERANCE: f64 = 1e-9;

    for (regime, config) in regimes() {
        for checkpoint in 1..=CHECKPOINTS {
            let elapsed = f64::from(checkpoint) * INTERVAL;

            // Ground truth: one analytical jump across the whole interval.
            let mut reference = step(config);
            reference.advance(elapsed);

            for rate in RATES {
                let mut spring = step(config);
                advance_for(&mut spring, elapsed, rate);

                let at = format!("{regime} at t={elapsed} stepped at {rate} fps");
                assert_close(spring.value(), reference.value(), TOLERANCE, &at);
                assert_close(spring.velocity(), reference.velocity(), TOLERANCE, &at);
            }
        }
    }
}

/// Frame-rate independence alone only proves the solver is self-consistent — a
/// solver that always returned zero would pass it. This pins the trajectory to
/// the physics: feed the solver's own output back into
/// `x'' + 2ζω₀x' + ω₀²x = 0` and check the residual vanishes, and check the
/// reported velocity really is dx/dt.
#[test]
fn analytical_solution_satisfies_the_equation_of_motion() {
    const H: f64 = 1e-4;
    const TOLERANCE: f64 = 1e-4;

    /// Displacement from the target `t` seconds into a unit step.
    fn displacement_at(config: SpringConfig, t: f64) -> f64 {
        let mut spring = step(config);
        spring.advance(t);
        spring.value() - spring.target()
    }

    for (regime, config) in regimes() {
        let (zeta, omega) = (config.damping_ratio(), config.angular_frequency());

        for checkpoint in 1..=6 {
            let t = f64::from(checkpoint) * 0.05;

            let previous = displacement_at(config, t - H);
            let current = displacement_at(config, t);
            let next = displacement_at(config, t + H);

            // Central differences.
            let first = (next - previous) / (2.0 * H);
            let second = (next - 2.0 * current + previous) / (H * H);

            let residual = second + 2.0 * zeta * omega * first + omega * omega * current;
            assert_close(
                residual,
                0.0,
                TOLERANCE,
                &format!("{regime}: equation of motion residual at t={t}"),
            );

            let mut spring = step(config);
            spring.advance(t);
            assert_close(
                spring.velocity(),
                first,
                TOLERANCE,
                &format!("{regime}: reported velocity vs dx/dt at t={t}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Degenerate states
// ---------------------------------------------------------------------------

/// value == target and velocity == 0 is a fixed point. Nothing may ever happen,
/// no matter how long or how coarsely it is stepped.
#[test]
fn at_equilibrium_stays_at_equilibrium() {
    for (regime, config) in regimes() {
        let mut spring = Spring::new(42.0).with_config(config);
        assert_eq!(spring.value(), spring.target());
        assert_eq!(spring.velocity(), 0.0);

        for _ in 0..600 {
            spring.advance(1.0 / 60.0);
            assert_eq!(spring.value(), 42.0, "{regime}: drifted off equilibrium");
            assert_eq!(spring.velocity(), 0.0, "{regime}: invented velocity");
        }

        // Including a single absurdly long frame.
        spring.advance(1_000.0);
        assert_eq!(spring.value(), 42.0, "{regime}: drifted over a long frame");
        assert_eq!(spring.velocity(), 0.0, "{regime}: invented velocity");
    }
}

/// Displacement 0 with velocity 100 — the state a fling produces when the
/// gesture releases exactly on target. It is also the input that divides by
/// zero in any solver that normalises by the displacement, so the whole
/// trajectory is checked for NaN and infinity rather than just the endpoint.
#[test]
fn at_target_with_nonzero_velocity_remains_finite() {
    for (regime, config) in regimes() {
        let mut spring = Spring::new(0.0f64).with_config(config).with_velocity(100.0);
        assert_eq!(spring.value() - spring.target(), 0.0);

        let mut excursion = 0.0f64;
        for frame in 0..600 {
            spring.advance(1.0 / 60.0);

            assert!(
                spring.value().is_finite(),
                "{regime}: value became {} on frame {frame}",
                spring.value()
            );
            assert!(
                spring.velocity().is_finite(),
                "{regime}: velocity became {} on frame {frame}",
                spring.velocity()
            );

            excursion = excursion.max(spring.value().abs());
        }

        // Finite is not enough: the kick has to actually throw the spring off
        // the target, and ten seconds later it has to be back at rest on it.
        assert!(
            excursion > 1.0,
            "{regime}: a velocity of 100 only moved the spring by {excursion}"
        );
        assert!(spring.is_settled(), "{regime}: never came back to rest");
        assert_eq!(spring.value(), 0.0, "{regime}: settled off target");
    }
}

/// `dt == 0` is a whole frame of nothing: a paused animation, or a duplicated
/// timestamp. It must be a no-op, not a division by zero or a lost frame.
#[test]
fn zero_dt_changes_nothing() {
    let mut spring = step(underdamped());
    advance_for(&mut spring, 0.1, 240.0);

    let (value, velocity) = (spring.value(), spring.velocity());
    assert!(velocity > 0.0, "the fixture should be in motion");

    for _ in 0..100 {
        spring.advance(0.0);
    }
    assert_eq!(spring.value(), value);
    assert_eq!(spring.velocity(), velocity);

    // Nor may zero-length frames perturb a trajectory they are interleaved into.
    let mut interleaved = step(underdamped());
    let mut continuous = step(underdamped());
    for _ in 0..240 {
        interleaved.advance(0.0);
        interleaved.advance(1.0 / 240.0);
        continuous.advance(1.0 / 240.0);
    }
    assert_eq!(interleaved.value(), continuous.value());
    assert_eq!(interleaved.velocity(), continuous.velocity());
}

/// Clocks go backwards and timers return garbage; neither may corrupt a spring.
#[test]
fn negative_or_non_finite_dt_is_ignored() {
    for dt in [
        -1.0 / 60.0,
        -1_000.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let mut spring = step(underdamped());
        advance_for(&mut spring, 0.1, 240.0);

        let (value, velocity) = (spring.value(), spring.velocity());
        spring.advance(dt);

        assert_eq!(spring.value(), value, "dt = {dt} moved the spring");
        assert_eq!(
            spring.velocity(),
            velocity,
            "dt = {dt} changed the velocity"
        );
    }
}

// ---------------------------------------------------------------------------
// Retargeting
// ---------------------------------------------------------------------------

/// Retargeting mid-flight is the whole point of a spring in a UI: the goal
/// posts move, the moving object does not. Momentum has to carry across.
#[test]
fn retargeting_does_not_reset_velocity() {
    let mut spring = step(underdamped());
    advance_for(&mut spring, 0.1, 240.0);

    let (value, velocity) = (spring.value(), spring.velocity());
    assert!(velocity > 0.0, "the fixture should be in motion");

    spring.set_target(5.0);
    assert_eq!(
        spring.velocity(),
        velocity,
        "retargeting reset the velocity"
    );
    assert_eq!(spring.value(), value, "retargeting teleported the value");
    assert_eq!(spring.target(), 5.0);

    // The velocity also has to survive the next `advance`, not just the
    // assignment: over a very short frame it can barely change.
    spring.advance(1e-4);
    assert_close(
        spring.velocity(),
        velocity,
        0.1,
        "velocity immediately after retargeting",
    );

    // And a spring constructed directly in the post-retarget state has to
    // evolve identically — the retarget carries the complete state, nothing
    // hidden is reset along with it.
    let mut equivalent = Spring::new(value)
        .with_target(5.0)
        .with_velocity(velocity)
        .with_config(underdamped());

    let mut retargeted = step(underdamped());
    advance_for(&mut retargeted, 0.1, 240.0);
    retargeted.set_target(5.0);

    advance_for(&mut retargeted, 0.2, 240.0);
    advance_for(&mut equivalent, 0.2, 240.0);

    assert_eq!(retargeted.value(), equivalent.value());
    assert_eq!(retargeted.velocity(), equivalent.velocity());
}

// ---------------------------------------------------------------------------
// Damping regimes
// ---------------------------------------------------------------------------

/// ζ = 1 is the fastest approach that never crosses the target. A UI that asks
/// for a critical spring is asking for exactly that guarantee.
#[test]
fn critically_damped_spring_does_not_overshoot() {
    let samples = step_response(critically_damped());

    assert!(
        peak(&samples) <= 1.0 + 1e-12,
        "critically damped spring reached {}",
        peak(&samples)
    );
    assert!(
        rises_monotonically(&samples),
        "critically damped spring moved backwards"
    );
    assert_eq!(*samples.last().unwrap(), 1.0, "never settled on the target");
}

/// ζ = 0.3 has to bounce, and by the textbook amount: the first peak of a step
/// response overshoots by exp(-πζ / √(1 - ζ²)) ≈ 37%.
#[test]
fn underdamped_spring_overshoots() {
    let samples = step_response(underdamped());

    let zeta = 0.3f64;
    let expected = 1.0 + (-std::f64::consts::PI * zeta / (1.0 - zeta * zeta).sqrt()).exp();
    assert_close(peak(&samples), expected, 5e-3, "first peak");

    // A real oscillation comes back the other way before settling.
    let trough = samples.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(trough < 1.0, "spring never crossed back under the target");
    assert_eq!(*samples.last().unwrap(), 1.0, "never settled on the target");
}

/// ζ = 2 is slower than critical but equally forbidden from crossing.
#[test]
fn overdamped_spring_does_not_overshoot() {
    let samples = step_response(overdamped());

    assert!(
        peak(&samples) <= 1.0 + 1e-12,
        "overdamped spring reached {}",
        peak(&samples)
    );
    assert!(
        rises_monotonically(&samples),
        "overdamped spring moved backwards"
    );
    assert_eq!(*samples.last().unwrap(), 1.0, "never settled on the target");

    // Overdamped must also be visibly lazier than critical over the same span.
    let critical = step_response(critically_damped());
    assert!(
        samples[200] < critical[200],
        "overdamped spring was not slower than the critical one"
    );
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// `duration`/`bounce` are the ergonomic front door to a config, so they have
/// to accept the ordinary values and agree with the all-at-once constructor.
#[test]
fn duration_and_bounce_build_the_same_config_as_from_duration_bounce() {
    let built = SpringConfig::new().duration(0.4).bounce(0.25);
    let direct = SpringConfig::from_duration_bounce(0.4, 0.25);

    assert_eq!(built.angular_frequency(), direct.angular_frequency());
    assert_eq!(built.damping_ratio(), direct.damping_ratio());
    assert_close(
        built.angular_frequency(),
        std::f64::consts::TAU / 0.4,
        1e-12,
        "ω₀ from a 0.4 s duration",
    );

    // Order must not matter, and neither setter may disturb the other.
    let reversed = SpringConfig::new().bounce(0.25).duration(0.4);
    assert_eq!(reversed.angular_frequency(), built.angular_frequency());
    assert_eq!(reversed.damping_ratio(), built.damping_ratio());
}

/// Whether `build` returns rather than panicking.
///
/// The panic hook is silenced for the call so an expected rejection does not
/// litter the test output, and restored before returning so a genuine failure
/// still prints normally.
fn is_accepted(build: impl FnOnce() -> SpringConfig + std::panic::UnwindSafe) -> bool {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let outcome = std::panic::catch_unwind(build);

    std::panic::set_hook(previous_hook);

    outcome.is_ok()
}

/// The degenerate configs have to be rejected at construction rather than
/// producing a spring that never arrives: ω₀ = 0 drifts in a straight line for
/// ever, and NaN poisons every value that touches it.
#[test]
fn duration_rejects_non_finite_and_non_positive_values() {
    for duration in [0.0, -0.5, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(
            !is_accepted(move || SpringConfig::new().duration(duration)),
            "duration() accepted {duration}"
        );
    }
}

/// `response` is a duration under another name, so it has to reject the same
/// values — infinity in particular, which passes a bare `> 0.0` test and turns
/// into ω₀ = TAU / ∞ = 0.
///
/// A zero ω₀ is not merely slow. The underdamped branch divides by
/// ω_d = ω₀√(1 - ζ²) and the overdamped branch divides by r₁ - r₂, both of
/// which are zero, so two of the three regimes hand back NaN. The third
/// degenerates into `x' = x + v·dt` — a free particle that never arrives.
#[test]
fn from_response_damping_rejects_non_finite_and_non_positive_values() {
    for response in [0.0, -0.5, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(
            !is_accepted(move || SpringConfig::from_response_damping(response, 1.0)),
            "from_response_damping() accepted a response of {response}"
        );
    }

    // An infinite damping ratio is NaN on arrival: the overdamped branch
    // computes -ζω₀ + ω₀√(ζ² - 1), which for an infinite ζ is -∞ + ∞.
    for damping_ratio in [-0.5, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(
            !is_accepted(move || SpringConfig::from_response_damping(0.5, damping_ratio)),
            "from_response_damping() accepted a damping ratio of {damping_ratio}"
        );
    }

    // Ordinary values still land where they should.
    let config = SpringConfig::from_response_damping(0.4, 0.6);
    assert_close(
        config.angular_frequency(),
        std::f64::consts::TAU / 0.4,
        1e-12,
        "ω₀ from a 0.4 s response",
    );
    assert_close(config.damping_ratio(), 0.6, 1e-12, "ζ");
}

/// The bug this guards against was not one constructor being wrong; it was the
/// constructors disagreeing. `duration`, `from_duration_bounce` and
/// `from_physical` all rejected an infinite parameter while
/// `from_response_damping` waved it through, so whether a bad config was caught
/// depended on which spelling the caller happened to reach for — and the
/// builders are further entry points that must not become a way around it.
#[test]
fn every_constructor_rejects_a_non_finite_parameter() {
    type Constructor = fn(f64) -> SpringConfig;

    const ENTRY_POINTS: [(&str, Constructor); 14] = [
        ("duration", |bad| SpringConfig::new().duration(bad)),
        ("bounce", |bad| SpringConfig::new().bounce(bad)),
        ("from_duration_bounce duration", |bad| {
            SpringConfig::from_duration_bounce(bad, 0.0)
        }),
        ("from_duration_bounce bounce", |bad| {
            SpringConfig::from_duration_bounce(0.3, bad)
        }),
        ("from_response_damping response", |bad| {
            SpringConfig::from_response_damping(bad, 1.0)
        }),
        ("from_response_damping damping", |bad| {
            SpringConfig::from_response_damping(0.3, bad)
        }),
        ("from_physical mass", |bad| {
            SpringConfig::from_physical(bad, 100.0, 10.0)
        }),
        ("from_physical stiffness", |bad| {
            SpringConfig::from_physical(1.0, bad, 10.0)
        }),
        ("from_physical damping", |bad| {
            SpringConfig::from_physical(1.0, 100.0, bad)
        }),
        ("responsive().response", |bad| {
            SpringConfig::responsive().response(bad).build()
        }),
        ("responsive().damping", |bad| {
            SpringConfig::responsive().damping(bad).build()
        }),
        ("physical().mass", |bad| {
            SpringConfig::physical().mass(bad).build()
        }),
        ("physical().stiffness", |bad| {
            SpringConfig::physical().stiffness(bad).build()
        }),
        ("physical().damping", |bad| {
            SpringConfig::physical().damping(bad).build()
        }),
    ];

    for bad in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        for (name, construct) in ENTRY_POINTS {
            assert!(
                !is_accepted(move || construct(bad)),
                "{name} accepted {bad}"
            );
        }
    }
}

/// ζ = 0 is an undamped spring: mathematically the best behaved case in the
/// solver — the transition matrix is a pure rotation and energy is conserved to
/// a part in 10¹² — but it never stops. `is_settled` is false for ever, so a
/// render loop written as `while !spring.is_settled()` never exits and whatever
/// drives it is never released.
///
/// It is reachable by several different spellings, and the point of this test
/// is that they agree. Before this, `bounce(1.0)` was rejected while
/// `from_duration_bounce(0.3, 1.0)` — the same request, differently phrased —
/// went through.
#[test]
fn every_constructor_rejects_an_undamped_spring() {
    type Door = fn() -> SpringConfig;

    let doors: [(&str, Door); 6] = [
        ("bounce(1.0)", || SpringConfig::new().bounce(1.0)),
        ("from_duration_bounce(_, 1.0)", || {
            SpringConfig::from_duration_bounce(0.3, 1.0)
        }),
        ("from_response_damping(_, 0.0)", || {
            SpringConfig::from_response_damping(0.3, 0.0)
        }),
        ("from_physical(_, _, 0.0)", || {
            SpringConfig::from_physical(1.0, 100.0, 0.0)
        }),
        ("responsive().damping(0.0)", || {
            SpringConfig::responsive().damping(0.0).build()
        }),
        ("physical().damping(0.0)", || {
            SpringConfig::physical().damping(0.0).build()
        }),
    ];

    for (name, construct) in doors {
        assert!(
            !is_accepted(construct),
            "{name} built an undamped spring, which never settles"
        );
    }

    // A hair above zero is still a legal spring, however lightly damped: the
    // rule is "damped at all", not "damped enough to look sensible".
    let barely = SpringConfig::from_response_damping(0.3, 1e-9);
    assert!(barely.damping_ratio() > 0.0);

    let mut spring = Spring::new(0.0f64)
        .with_target(1.0)
        .with_config(SpringConfig::new().duration(0.3).bounce(0.999));
    advance_for(&mut spring, 2.0, 240.0);
    assert!(spring.value().is_finite() && spring.velocity().is_finite());
}

/// The property all of that validation exists to protect: nothing a caller can
/// legally construct may put a NaN into a trajectory.
///
/// NaN is the failure that matters because it never compares true to anything,
/// so `is_settled` stays false for ever and a render loop written as
/// `while !spring.is_settled()` never stops — with a value that rasterises to
/// nothing. The sweep deliberately straddles the ζ = 1 branch boundary, where
/// ω_d and r₁ - r₂ both approach zero.
#[test]
fn no_valid_config_produces_a_non_finite_trajectory() {
    const DAMPING_RATIOS: [f64; 9] = [
        1e-6, 0.05, 0.5, 0.999_9, 0.999_99, 1.0, 1.000_01, 1.000_1, 6.0,
    ];

    for response in [0.05, 0.2, 1.0, 4.0] {
        for damping_ratio in DAMPING_RATIOS {
            let config = SpringConfig::from_response_damping(response, damping_ratio);
            assert!(
                config.angular_frequency().is_finite() && config.angular_frequency() > 0.0,
                "response {response}: ω₀ is {}",
                config.angular_frequency()
            );

            let mut spring = Spring::new(0.0f64)
                .with_target(1.0)
                .with_velocity(25.0)
                .with_config(config);

            for frame in 0..600 {
                spring.advance(1.0 / 60.0);

                assert!(
                    spring.value().is_finite() && spring.velocity().is_finite(),
                    "response {response}, ζ {damping_ratio}: value {} velocity {} on frame {frame}",
                    spring.value(),
                    spring.velocity()
                );
            }
        }
    }
}

/// A config built through `duration` must produce a spring that actually
/// converges — the symptom an ω₀ = 0 config would show.
#[test]
fn duration_produces_a_spring_that_settles() {
    let mut spring = Spring::new(0.0)
        .with_target(1.0)
        .with_config(SpringConfig::new().duration(0.4).bounce(0.0));

    advance_for(&mut spring, 5.0, 240.0);

    assert!(spring.is_settled(), "spring never settled");
    assert_eq!(spring.value(), 1.0);
    assert_eq!(spring.velocity(), 0.0);
}

/// Whatever the caller leaves unset in the physical builder has to keep meaning
/// what it means for a default config: the default duration and the default
/// bounce. Setting only the mass must not quietly turn the default critical
/// spring into a bouncy one, because stiffness and damping are only meaningful
/// relative to the mass they act on.
#[test]
fn physical_builder_defaults_track_the_terms_that_were_set() {
    let default = SpringConfig::default();

    let matches_default = |config: SpringConfig, what: &str| {
        assert_close(config.damping_ratio(), default.damping_ratio(), 1e-12, what);
        assert_close(
            config.angular_frequency(),
            default.angular_frequency(),
            1e-12,
            what,
        );
    };

    matches_default(SpringConfig::physical().build(), "no terms set");

    // Only the mass: both derived terms follow it, so the feel is unchanged.
    for mass in [0.1, 0.5, 2.0, 50.0] {
        matches_default(
            SpringConfig::physical().mass(mass).build(),
            &format!("mass {mass} alone"),
        );
    }

    // An explicit stiffness moves ω₀ — that is the point of setting it — but an
    // unset damping still has to mean the default bounce at the new frequency.
    for (mass, stiffness) in [(2.0, 200.0), (1.0, 50.0), (4.0, 900.0)] {
        let config = SpringConfig::physical()
            .mass(mass)
            .stiffness(stiffness)
            .build();

        assert_close(
            config.damping_ratio(),
            1.0,
            1e-12,
            &format!("ζ with mass {mass}, stiffness {stiffness} and no damping"),
        );
        assert_close(
            config.angular_frequency(),
            (stiffness / mass).sqrt(),
            1e-12,
            &format!("ω₀ with mass {mass} and stiffness {stiffness}"),
        );
    }

    // Likewise an explicit damping with an unset stiffness: ω₀ stays default.
    let damped = SpringConfig::physical().mass(2.0).damping(20.0).build();
    assert_close(
        damped.angular_frequency(),
        default.angular_frequency(),
        1e-12,
        "ω₀ with an explicit damping only",
    );

    // And once every term is given, the builder is a pure pass-through.
    let built = SpringConfig::physical()
        .mass(2.0)
        .stiffness(200.0)
        .damping(8.0)
        .build();
    let direct = SpringConfig::from_physical(2.0, 200.0, 8.0);
    assert_eq!(built.damping_ratio(), direct.damping_ratio());
    assert_eq!(built.angular_frequency(), direct.angular_frequency());
}

/// Mass/stiffness/damping is the physicist's parameterisation; the solver wants
/// ζ and ω₀. Check the conversion, then check the converted numbers actually
/// reach the solver by measuring the resulting motion.
#[test]
fn physical_config_produces_expected_damping_and_frequency() {
    // m = 2 kg, k = 200 N/m, c = 8 N·s/m
    //   ω₀ = √(k / m)     = √100  = 10 rad/s
    //   ζ  = c / 2√(k·m)  = 8/40  = 0.2
    let config = SpringConfig::from_physical(2.0, 200.0, 8.0);
    assert_close(config.angular_frequency(), 10.0, 1e-12, "ω₀");
    assert_close(config.damping_ratio(), 0.2, 1e-12, "ζ");

    // The builder is just another spelling of the same conversion.
    let built = SpringConfig::physical()
        .mass(2.0)
        .stiffness(200.0)
        .damping(8.0)
        .build();
    assert_eq!(built.angular_frequency(), config.angular_frequency());
    assert_eq!(built.damping_ratio(), config.damping_ratio());

    // c = 2√(k·m) is the definition of critical damping, and c = 0 of none.
    let critical = SpringConfig::from_physical(2.0, 200.0, 2.0 * (200.0f64 * 2.0).sqrt());
    assert_close(critical.damping_ratio(), 1.0, 1e-12, "ζ at c = 2√(k·m)");
    // ... and zero damping is no longer a config at all: it is an undamped
    // spring, which never settles.
    assert!(!is_accepted(|| SpringConfig::from_physical(
        2.0, 200.0, 0.0
    )));

    // ω₀ scales as √(k/m): four times the stiffness is twice the frequency.
    let stiffer = SpringConfig::from_physical(2.0, 800.0, 8.0);
    assert_close(
        stiffer.angular_frequency(),
        20.0,
        1e-12,
        "ω₀ at 4× stiffness",
    );

    // Now the motion. A ζ = 0.2 step response peaks half a damped period in,
    // at t = π / ω_d, overshooting by exp(-πζ / √(1 - ζ²)).
    let omega_d = 10.0 * (1.0 - 0.2 * 0.2f64).sqrt();
    let expected_time = std::f64::consts::PI / omega_d;
    let expected_peak = 1.0 + (-std::f64::consts::PI * 0.2 / (1.0 - 0.2 * 0.2f64).sqrt()).exp();

    let dt = 1.0 / 10_000.0;
    let mut spring = step(config);
    let (mut peak_value, mut peak_time) = (f64::NEG_INFINITY, 0.0);
    for frame in 1..=10_000 {
        spring.advance(dt);
        if spring.value() > peak_value {
            peak_value = spring.value();
            peak_time = f64::from(frame) * dt;
        }
    }

    assert_close(peak_time, expected_time, 2.0 * dt, "time of first peak");
    assert_close(peak_value, expected_peak, 1e-3, "height of first peak");
}

// ---------------------------------------------------------------------------
// Settling
// ---------------------------------------------------------------------------

/// Counts the frames a 0 → `travel` step takes to settle, at 1 kHz.
fn frames_to_settle(config: SpringConfig, travel: f64, epsilon: f64) -> u32 {
    let mut spring = Spring::new(0.0)
        .with_target(travel)
        .with_config(config)
        .with_epsilon(epsilon);

    let mut frames = 0;
    while !spring.is_settled() {
        spring.advance(1.0 / 1_000.0);
        frames += 1;

        assert!(frames < 100_000, "spring never settled");
    }

    frames
}

/// The epsilon is in the units of whatever is being animated, which only the
/// caller knows. A coarser one has to settle sooner, and scaling the epsilon
/// with the distance has to reproduce the same animation — that is what makes
/// the same config usable for both an opacity and a pixel offset.
#[test]
fn epsilon_controls_when_the_spring_is_settled() {
    assert_eq!(Spring::new(0.0).epsilon(), 0.001, "default epsilon changed");

    let config = critically_damped();

    let coarse = frames_to_settle(config, 1.0, 0.1);
    let default = frames_to_settle(config, 1.0, 0.001);
    let fine = frames_to_settle(config, 1.0, 0.000_01);
    assert!(
        coarse < default && default < fine,
        "a coarser epsilon must settle sooner: {coarse} < {default} < {fine}"
    );

    // Same journey in different units: 1 unit at 0.001, 1000 units at 1.0.
    // Only the ratio matters, so these must be the same animation.
    let small = frames_to_settle(config, 1.0, 0.001);
    let large = frames_to_settle(config, 1_000.0, 1.0);
    assert_eq!(
        small, large,
        "scaling the travel and the epsilon together changed the animation"
    );
}

/// The velocity threshold is `epsilon · ω₀`, so both settle conditions trip at
/// the same moment. In the tail of a critical spring |v| → ω₀|x|, so a spring
/// with a fixed velocity threshold instead keeps running until |x| is a small
/// fraction of the epsilon — a stricter animation than the caller asked for,
/// and one whose strictness depends on the stiffness.
#[test]
fn the_velocity_threshold_scales_with_the_frequency() {
    const EPSILON: f64 = 0.001;

    for response in [0.2, 0.6, 2.0] {
        let config = SpringConfig::from_response_damping(response, 1.0);

        let mut spring = Spring::new(0.0f64)
            .with_target(1.0)
            .with_config(config)
            .with_epsilon(EPSILON);

        // The displacement on the last frame before the spring called it done.
        let mut displacement = 1.0;
        let mut frames = 0;
        while !spring.is_settled() {
            displacement = (spring.value() - spring.target()).abs();
            spring.advance(1.0 / 10_000.0);

            frames += 1;
            assert!(frames < 1_000_000, "spring never settled");
        }

        assert!(
            (0.5 * EPSILON..1.5 * EPSILON).contains(&displacement),
            "response {response}: stopped {displacement} from the target, \
             expected about {EPSILON} — the two thresholds are not tripping together"
        );
    }
}

// ---------------------------------------------------------------------------
// Generic values
// ---------------------------------------------------------------------------

/// A second value type, so the generic machinery is exercised by something that
/// is not `f64` with extra steps: a 2D point whose delta is a 2D vector and
/// whose magnitude is a euclidean length rather than an absolute value.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl SpringDelta for Vec2 {
    fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    fn scale(self, scalar: f64) -> Self {
        Self::new(self.x * scalar, self.y * scalar)
    }

    fn magnitude(self) -> f64 {
        self.x.hypot(self.y)
    }
}

impl SpringValue for Vec2 {
    type Delta = Self;

    fn displacement_from(self, target: Self) -> Self::Delta {
        Self::new(self.x - target.x, self.y - target.y)
    }

    fn add_displacement(self, displacement: Self::Delta) -> Self {
        self.add(displacement)
    }
}

/// The solver applies the same scalar coefficients to every component, so a 2D
/// spring has to be exactly two independent 1D springs. Any per-component
/// coupling — a normalisation, a magnitude sneaking into the maths — shows up
/// here as a divergence between the axes.
#[test]
fn a_two_dimensional_spring_is_two_scalar_springs() {
    for (regime, config) in regimes() {
        // A tiny epsilon keeps the settle snap out of the comparison: it uses a
        // euclidean length for the point and a per-axis one for the scalars, so
        // the two legitimately stop at slightly different moments.
        let epsilon = 1e-12;

        let mut point = Spring::new(Vec2::new(0.0, 0.0))
            .with_target(Vec2::new(3.0, -4.0))
            .with_velocity(Vec2::new(1.5, 2.5))
            .with_config(config)
            .with_epsilon(epsilon);

        let mut x = Spring::new(0.0)
            .with_target(3.0)
            .with_velocity(1.5)
            .with_config(config)
            .with_epsilon(epsilon);

        let mut y = Spring::new(0.0)
            .with_target(-4.0)
            .with_velocity(2.5)
            .with_config(config)
            .with_epsilon(epsilon);

        for frame in 0..240 {
            for spring in [&mut x, &mut y] {
                spring.advance(1.0 / 240.0);
            }
            point.advance(1.0 / 240.0);

            let at = format!("{regime} on frame {frame}");
            assert_eq!(
                point.value(),
                Vec2::new(x.value(), y.value()),
                "value, {at}"
            );
            assert_eq!(
                point.velocity(),
                Vec2::new(x.velocity(), y.velocity()),
                "velocity, {at}"
            );
        }
    }
}

/// Settling for a non-scalar value is a question about the length of the
/// displacement, not about any one component.
#[test]
fn a_two_dimensional_spring_settles_on_its_target() {
    let mut spring = Spring::new(Vec2::new(0.0, 0.0))
        .with_target(Vec2::new(3.0, -4.0))
        .with_config(critically_damped())
        .with_epsilon(0.001);

    // A displacement of (3, -4) has length 5, so it starts far from settled
    // even though neither component is especially large.
    assert!(!spring.is_settled());

    for _ in 0..1_200 {
        spring.advance(1.0 / 240.0);
    }

    assert!(spring.is_settled(), "never settled");
    assert_eq!(spring.value(), Vec2::new(3.0, -4.0), "settled off target");
    assert_eq!(spring.velocity(), Vec2::zero(), "settled still moving");
}

// ---------------------------------------------------------------------------
// Arrays
// ---------------------------------------------------------------------------

/// The const-generic array impl has to behave like the hand-written `Vec2`
/// above: one independent scalar spring per component, whatever the length.
#[test]
fn an_array_spring_is_a_bundle_of_scalar_springs() {
    const STARTS: [f64; 3] = [0.0, 5.0, -2.0];
    const TARGETS: [f64; 3] = [3.0, -4.0, 8.0];
    const VELOCITIES: [f64; 3] = [1.5, 2.5, -0.5];

    let config = underdamped();
    let epsilon = 1e-12;

    let mut bundle = Spring::new(STARTS)
        .with_target(TARGETS)
        .with_velocity(VELOCITIES)
        .with_config(config)
        .with_epsilon(epsilon);

    let mut scalars: Vec<Spring<f64>> = (0..3)
        .map(|axis| {
            Spring::new(STARTS[axis])
                .with_target(TARGETS[axis])
                .with_velocity(VELOCITIES[axis])
                .with_config(config)
                .with_epsilon(epsilon)
        })
        .collect();

    for frame in 0..240 {
        bundle.advance(1.0 / 240.0);
        for scalar in &mut scalars {
            scalar.advance(1.0 / 240.0);
        }

        for (axis, scalar) in scalars.iter().enumerate() {
            let at = format!("component {axis} on frame {frame}");
            assert_eq!(bundle.value()[axis], scalar.value(), "value, {at}");
            assert_eq!(bundle.velocity()[axis], scalar.velocity(), "velocity, {at}");
        }
    }
}

/// Settling for an array is about the euclidean distance left to travel, not
/// about any single component.
#[test]
fn an_array_spring_settles_on_the_euclidean_distance() {
    // (3, -4) is 5 away, so a spring with an epsilon of 4 is not there yet even
    // though neither component is more than 4 out on its own.
    let spring = Spring::new([0.0, 0.0])
        .with_target([3.0, -4.0])
        .with_config(critically_damped())
        .with_epsilon(4.0);
    assert!(!spring.is_settled());

    let mut spring = spring.with_epsilon(0.001);
    for _ in 0..1_200 {
        spring.advance(1.0 / 240.0);
    }

    assert!(spring.is_settled(), "never settled");
    assert_eq!(spring.value(), [3.0, -4.0]);
    assert_eq!(spring.velocity(), [0.0, 0.0]);
}

/// The same impls cover `f32`, scalar and array alike.
#[test]
fn f32_springs_settle_on_their_target() {
    let mut scalar = Spring::new(0.0f32)
        .with_target(1.0f32)
        .with_config(critically_damped());

    let mut array = Spring::new([0.0f32, 0.0])
        .with_target([3.0f32, -4.0])
        .with_config(critically_damped());

    for _ in 0..600 {
        scalar.advance(1.0 / 60.0);
        array.advance(1.0 / 60.0);
    }

    assert!(scalar.is_settled() && array.is_settled(), "never settled");
    assert_eq!(scalar.value(), 1.0f32);
    assert_eq!(array.value(), [3.0f32, -4.0]);
}

// ---------------------------------------------------------------------------
// Angles
// ---------------------------------------------------------------------------

/// The point of the type: 359° → 2° is a 3° nudge forwards, never a 357°
/// journey the other way round. Summing the per-frame steps distinguishes the
/// two unambiguously — the long way would total -357.
#[test]
fn an_angle_spring_takes_the_short_way_round() {
    let cases: [(f64, f64, f64); 6] = [
        (359.0, 2.0, 3.0),
        (2.0, 359.0, -3.0),
        (350.0, 10.0, 20.0),
        (10.0, 350.0, -20.0),
        (0.0, 90.0, 90.0),
        (270.0, 45.0, 135.0),
    ];

    for (start, target, expected) in cases {
        // Critically damped, so the spring never legitimately reverses and any
        // backwards step is the wrap picking the wrong arc.
        let mut spring = Spring::new(Angle::from_degrees(start))
            .with_target(Angle::from_degrees(target))
            .with_config(SpringConfig::from_response_damping(0.4, 1.0))
            .with_epsilon(1e-9);

        let mut travelled = 0.0;
        let mut previous = spring.value();

        for _ in 0..4_000 {
            spring.advance(1.0 / 1_000.0);

            let stepped = spring.value().displacement_from(previous).to_degrees();
            assert!(
                stepped * expected.signum() >= -1e-9,
                "{start}° → {target}°: stepped {stepped}°, the wrong way"
            );

            travelled += stepped;
            previous = spring.value();
        }

        assert_close(
            travelled,
            expected,
            1e-6,
            &format!("total rotation from {start}° to {target}°"),
        );
        assert_close(
            spring.value().degrees(),
            target,
            1e-6,
            &format!("final angle from {start}°"),
        );
    }
}

/// Angles are normalised on construction and stay normalised as they animate,
/// so `degrees()` is always a number you can hand straight to a transform.
#[test]
fn angles_stay_normalised() {
    assert_close(Angle::from_degrees(370.0).degrees(), 10.0, 1e-9, "370°");
    assert_close(Angle::from_degrees(-90.0).degrees(), 270.0, 1e-9, "-90°");
    assert_close(Angle::from_degrees(720.0).degrees(), 0.0, 1e-9, "720°");

    // A hard kick spins the angle through several rotations; every reported
    // value must still be in range.
    let mut spring = Spring::new(Angle::from_degrees(0.0))
        .with_target(Angle::from_degrees(90.0))
        .with_config(SpringConfig::from_response_damping(0.5, 0.4));
    spring.add_velocity(60.0);

    for _ in 0..2_000 {
        spring.advance(1.0 / 200.0);

        let degrees = spring.value().degrees();
        assert!(
            (0.0..360.0).contains(&degrees),
            "angle escaped its range: {degrees}°"
        );
    }

    assert!(spring.is_settled(), "never settled");
    assert_close(spring.value().degrees(), 90.0, 1e-9, "final angle");
}

/// A spring holds the only copy of its config, so it has to hand it back —
/// otherwise every caller that wants to display ζ or ω₀ keeps a duplicate
/// beside the spring, and the two can drift apart.
#[test]
fn a_spring_reports_the_config_it_was_built_with() {
    let config = SpringConfig::new().duration(0.42).bounce(0.3);
    let spring = Spring::new(0.0f64).with_config(config);

    assert_eq!(spring.config().damping_ratio(), config.damping_ratio());
    assert_eq!(
        spring.config().angular_frequency(),
        config.angular_frequency()
    );

    // The default is the default config, not something invented in `new`.
    let default = Spring::new(0.0f64);
    assert_eq!(
        default.config().angular_frequency(),
        SpringConfig::default().angular_frequency()
    );
    assert_eq!(
        default.config().damping_ratio(),
        SpringConfig::default().damping_ratio()
    );

    // And it tracks a replacement rather than reporting the original.
    let replaced = spring.with_config(SpringConfig::from_response_damping(0.9, 2.0));
    assert_close(
        replaced.config().angular_frequency(),
        std::f64::consts::TAU / 0.9,
        1e-12,
        "ω₀ after with_config",
    );
    assert_close(
        replaced.config().damping_ratio(),
        2.0,
        1e-12,
        "ζ after with_config",
    );

    // What the getter is actually for: the reported ω₀ is the one the settle
    // threshold uses, so a caller can predict when the spring will stop.
    let mut spring = Spring::new(0.0f64)
        .with_target(1.0)
        .with_config(config)
        .with_epsilon(0.01);
    advance_for(&mut spring, 3.0, 240.0);
    assert!(spring.is_settled());
    assert!(spring.velocity().abs() < spring.epsilon() * spring.config().angular_frequency());
}
