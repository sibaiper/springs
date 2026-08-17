//! Tests that hold the solver against textbook results for a damped harmonic
//! oscillator, rather than against its own behaviour.
//!
//! Where [`spring.rs`](spring.rs) checks that the API behaves, this checks that
//! the motion is the right motion: the amplitude decays by the logarithmic
//! decrement, the oscillation has the damped period, overshoot follows the
//! standard formula across the whole underdamped range, and energy only ever
//! leaves the system.

use springs::{Spring, SpringConfig};

/// ω₀ = 2π / 0.6 ≈ 10.47 rad/s for every fixture here.
const RESPONSE: f64 = 0.6;

/// Fine enough that peak positions are accurate to a fraction of a millisecond.
const DT: f64 = 1.0 / 10_000.0;

/// Small enough that the settle snap never fires inside a test's horizon, so
/// the raw trajectory is what gets measured.
const NO_SETTLING: f64 = 1e-12;

#[track_caller]
fn assert_close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    let error = (actual - expected).abs();
    assert!(
        error <= tolerance,
        "{what}: expected {expected}, got {actual} (off by {error:e}, tolerance {tolerance:e})"
    );
}

fn config(damping_ratio: f64) -> SpringConfig {
    SpringConfig::from_response_damping(RESPONSE, damping_ratio)
}

fn regimes() -> [(&'static str, SpringConfig); 3] {
    [
        ("underdamped", config(0.3)),
        ("critically damped", config(1.0)),
        ("overdamped", config(2.0)),
    ]
}

/// The state `t` seconds after starting at `displacement` with `velocity`,
/// as (displacement, velocity), reached in a single analytical jump.
fn response(config: SpringConfig, displacement: f64, velocity: f64, t: f64) -> (f64, f64) {
    let mut spring = Spring::new(displacement)
        .with_target(0.0)
        .with_velocity(velocity)
        .with_config(config)
        .with_epsilon(NO_SETTLING);
    spring.advance(t);

    (spring.value(), spring.velocity())
}

/// The free response sampled every [`DT`] for `seconds`, as displacement from
/// a target of zero.
fn free_response(config: SpringConfig, displacement: f64, seconds: f64) -> Vec<f64> {
    let mut spring = Spring::new(displacement)
        .with_target(0.0)
        .with_config(config)
        .with_epsilon(NO_SETTLING);

    (0..(seconds / DT) as usize)
        .map(|_| {
            spring.advance(DT);
            spring.value()
        })
        .collect()
}

/// Interior samples higher than both of their neighbours, as (index, value).
fn local_maxima(samples: &[f64]) -> Vec<(usize, f64)> {
    (1..samples.len().saturating_sub(1))
        .filter(|&i| samples[i] > samples[i - 1] && samples[i] >= samples[i + 1])
        .map(|i| (i, samples[i]))
        .collect()
}

// ---------------------------------------------------------------------------
// Linearity
// ---------------------------------------------------------------------------

/// The closed-form solution is linear in the initial state, which is exactly
/// what the solver's transition-matrix form assumes: it computes four scalars
/// and applies them to the state. So the response to (x₀, v₀) has to equal
/// x₀·(response to a unit displacement) + v₀·(response to a unit velocity).
///
/// If the regimes ever grow a term that is not linear in the initial state,
/// this fails even though every individual trajectory still looks plausible.
#[test]
fn the_solution_is_linear_in_the_initial_state() {
    const TOLERANCE: f64 = 1e-9;

    for (regime, config) in regimes() {
        for t in [0.01, 0.05, 0.2, 0.5] {
            let (unit_x_displacement, unit_x_velocity) = response(config, 1.0, 0.0, t);
            let (unit_v_displacement, unit_v_velocity) = response(config, 0.0, 1.0, t);

            for (x0, v0) in [(2.0, 0.0), (0.0, -3.0), (1.5, 7.0), (-4.0, 2.5)] {
                let (displacement, velocity) = response(config, x0, v0, t);
                let at = format!("{regime} from ({x0}, {v0}) at t={t}");

                assert_close(
                    displacement,
                    x0 * unit_x_displacement + v0 * unit_v_displacement,
                    TOLERANCE,
                    &format!("superposed displacement, {at}"),
                );
                assert_close(
                    velocity,
                    x0 * unit_x_velocity + v0 * unit_v_velocity,
                    TOLERANCE,
                    &format!("superposed velocity, {at}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Energy
// ---------------------------------------------------------------------------

/// A damper takes energy out and never puts it back. E = ½v² + ½ω₀²x² must
/// therefore decrease monotonically for any ζ > 0, in every regime — an
/// invariant that catches a sign error in the damping term even where the
/// trajectory still looks like a plausible decay.
#[test]
fn mechanical_energy_never_increases() {
    for (regime, config) in regimes() {
        let omega = config.angular_frequency();

        let mut spring = Spring::new(2.0)
            .with_target(0.0)
            .with_velocity(-5.0)
            .with_config(config)
            .with_epsilon(NO_SETTLING);

        let energy = |spring: &Spring<f64>| {
            0.5 * spring.velocity().powi(2) + 0.5 * omega * omega * spring.value().powi(2)
        };

        let mut previous = energy(&spring);
        let initial = previous;

        for frame in 0..20_000 {
            spring.advance(DT);
            let current = energy(&spring);

            assert!(
                current <= previous + 1e-12 * initial,
                "{regime}: energy rose from {previous} to {current} on frame {frame}"
            );
            previous = current;
        }

        // And it actually went somewhere, rather than being trivially constant.
        // The bound is loose on purpose: the slowest root here is the
        // overdamped one at r₁ ≈ -2.8, which still leaves ~1e-5 of the energy
        // after two seconds.
        assert!(
            previous < initial * 1e-4,
            "{regime}: energy barely dissipated ({initial} to {previous})"
        );
    }
}

// ---------------------------------------------------------------------------
// Underdamped motion
// ---------------------------------------------------------------------------

/// Successive peaks of a free underdamped response fall by a fixed ratio, the
/// logarithmic decrement: x(n+1)/x(n) = exp(-2πζ / √(1 - ζ²)). This pins ζ
/// through the *shape* of the decay rather than through the config getter.
#[test]
fn successive_peaks_decay_by_the_logarithmic_decrement() {
    for zeta in [0.1, 0.2, 0.3, 0.5] {
        let samples = free_response(config(zeta), 1.0, 3.0);
        let peaks = local_maxima(&samples);

        assert!(
            peaks.len() >= 3,
            "ζ = {zeta}: expected at least three peaks, found {}",
            peaks.len()
        );

        let expected = (-std::f64::consts::TAU * zeta / (1.0 - zeta * zeta).sqrt()).exp();

        for pair in peaks.windows(2).take(2) {
            let ratio = pair[1].1 / pair[0].1;
            assert_close(
                ratio,
                expected,
                expected * 1e-3,
                &format!("ζ = {zeta}: peak ratio"),
            );
        }
    }
}

/// The gap between those peaks is the damped period, 2π / ω_d — which is a
/// different quantity from ω₀ whenever the spring is damped at all.
#[test]
fn the_oscillation_period_is_the_damped_period() {
    for zeta in [0.1, 0.2, 0.3, 0.5] {
        let spring_config = config(zeta);
        let omega_d = spring_config.angular_frequency() * (1.0 - zeta * zeta).sqrt();
        let expected = std::f64::consts::TAU / omega_d;

        let samples = free_response(spring_config, 1.0, 3.0);
        let peaks = local_maxima(&samples);

        for pair in peaks.windows(2).take(2) {
            let measured = (pair[1].0 - pair[0].0) as f64 * DT;
            assert_close(
                measured,
                expected,
                3.0 * DT,
                &format!("ζ = {zeta}: damped period"),
            );
        }
    }
}

/// The first peak of a step response overshoots by exp(-πζ / √(1 - ζ²)) —
/// checked across the underdamped range rather than at a single ζ, so the
/// relationship is pinned and not just one lucky point on it.
#[test]
fn overshoot_matches_theory_across_the_underdamped_range() {
    for zeta in [0.05, 0.1, 0.2, 0.3, 0.5, 0.7, 0.9] {
        let mut spring = Spring::new(0.0)
            .with_target(1.0)
            .with_config(config(zeta))
            .with_epsilon(NO_SETTLING);

        let mut peak = f64::NEG_INFINITY;
        for _ in 0..30_000 {
            spring.advance(DT);
            peak = peak.max(spring.value());
        }

        let expected = 1.0 + (-std::f64::consts::PI * zeta / (1.0 - zeta * zeta).sqrt()).exp();
        assert_close(peak, expected, 1e-6, &format!("ζ = {zeta}: first peak"));
    }
}

/// Overshoot has to fall monotonically as damping rises, all the way to none
/// at ζ = 1. A regime boundary that is subtly discontinuous shows up as a
/// kink here even when each side is individually correct.
#[test]
fn overshoot_falls_monotonically_with_damping() {
    let mut previous = f64::INFINITY;

    for step in 0..=40 {
        let zeta = 0.02 + f64::from(step) * 0.0245; // 0.02 up to 1.0
        let mut spring = Spring::new(0.0)
            .with_target(1.0)
            .with_config(config(zeta))
            .with_epsilon(NO_SETTLING);

        let mut peak = f64::NEG_INFINITY;
        for _ in 0..20_000 {
            spring.advance(DT);
            peak = peak.max(spring.value());
        }

        assert!(
            peak <= previous + 1e-9,
            "overshoot rose from {previous} to {peak} at ζ = {zeta}"
        );
        previous = peak;
    }

    assert_close(previous, 1.0, 1e-6, "overshoot at ζ = 1");
}

// ---------------------------------------------------------------------------
// Precision and conversions
// ---------------------------------------------------------------------------

/// The `f32` and `f64` springs are the same solver with a narrower state, so
/// they should track each other to `f32` precision. This is the test that would
/// catch a `scale` implementation that rounds through the wrong type.
#[test]
fn f32_and_f64_springs_track_each_other() {
    let spring_config = config(0.3);

    let mut wide = Spring::new(0.0f64)
        .with_target(1.0)
        .with_config(spring_config);
    let mut narrow = Spring::new(0.0f32)
        .with_target(1.0)
        .with_config(spring_config);

    let mut compared = 0;
    for _ in 0..600 {
        // Stop before either snaps: they settle a frame apart, and the snap is
        // a jump of up to one epsilon that has nothing to do with precision.
        if wide.is_settled() || narrow.is_settled() {
            break;
        }

        wide.advance(1.0 / 60.0);
        narrow.advance(1.0 / 60.0);
        compared += 1;

        assert_close(
            f64::from(narrow.value()),
            wide.value(),
            1e-4,
            &format!("f32 vs f64 after {compared} frames"),
        );
    }

    assert!(compared > 60, "only compared {compared} frames");
}

/// The three ways of describing a spring have to name the same spring. Given a
/// duration and a bounce, both the responsive and the physical constructors
/// must land on the same ζ and ω₀.
#[test]
fn every_constructor_can_describe_the_same_spring() {
    const TOLERANCE: f64 = 1e-12;

    for duration in [0.15, 0.3, 0.8, 2.0] {
        for bounce in [-0.5, 0.0, 0.4, 0.9] {
            let original = SpringConfig::from_duration_bounce(duration, bounce);
            let (zeta, omega) = (original.damping_ratio(), original.angular_frequency());
            let at = format!("duration {duration}, bounce {bounce}");

            // Response/damping is the same numbers under different names.
            let responsive = SpringConfig::from_response_damping(duration, 1.0 - bounce);
            assert_close(
                responsive.damping_ratio(),
                zeta,
                TOLERANCE,
                &format!("ζ, {at}"),
            );
            assert_close(
                responsive.angular_frequency(),
                omega,
                TOLERANCE,
                &format!("ω₀, {at}"),
            );

            // And any mass can reproduce it, given the matching k and c.
            for mass in [0.25, 1.0, 7.5] {
                let physical = SpringConfig::from_physical(
                    mass,
                    mass * omega * omega,
                    2.0 * zeta * mass * omega,
                );

                assert_close(
                    physical.damping_ratio(),
                    zeta,
                    TOLERANCE,
                    &format!("ζ via mass {mass}, {at}"),
                );
                assert_close(
                    physical.angular_frequency(),
                    omega,
                    TOLERANCE,
                    &format!("ω₀ via mass {mass}, {at}"),
                );
            }
        }
    }
}
