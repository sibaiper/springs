//! Tests for the optional `glam` integration.
//!
//! The whole file compiles away without the feature, so `cargo test` on a
//! default build neither needs nor mentions glam.
#![cfg(feature = "glam")]

use glam::{DVec2, Vec2, Vec3};
use springs::{Spring, SpringConfig};

fn oscillatory() -> SpringConfig {
    SpringConfig::from_response_damping(0.6, 0.3)
}

fn critical() -> SpringConfig {
    SpringConfig::from_response_damping(0.6, 1.0)
}

/// A glam vector and the built-in array impl are the same arithmetic in a
/// different container, so they must agree component for component. Anything
/// glam does differently — a fused multiply, a SIMD reassociation — shows up
/// here rather than as a mystery divergence later.
#[test]
fn glam_vectors_match_the_array_impls() {
    let epsilon = 1e-9;

    let mut vector = Spring::new(Vec2::new(0.0, 5.0))
        .with_target(Vec2::new(3.0, -4.0))
        .with_velocity(Vec2::new(1.5, 2.5))
        .with_config(oscillatory())
        .with_epsilon(epsilon);

    let mut array = Spring::new([0.0f32, 5.0])
        .with_target([3.0f32, -4.0])
        .with_velocity([1.5f32, 2.5])
        .with_config(oscillatory())
        .with_epsilon(epsilon);

    for frame in 0..240 {
        vector.advance(1.0 / 240.0);
        array.advance(1.0 / 240.0);

        let value = vector.value();
        let velocity = vector.velocity();

        assert_eq!([value.x, value.y], array.value(), "value on frame {frame}");
        assert_eq!(
            [velocity.x, velocity.y],
            array.velocity(),
            "velocity on frame {frame}"
        );
    }
}

/// Settling uses `Vec::length`, so it is the distance still to travel rather
/// than any single component.
#[test]
fn a_glam_spring_settles_on_the_euclidean_distance() {
    // (3, -4) is 5 away, so an epsilon of 4 is not yet arrived even though
    // neither component is more than 4 out on its own.
    let spring = Spring::new(DVec2::ZERO)
        .with_target(DVec2::new(3.0, -4.0))
        .with_config(critical())
        .with_epsilon(4.0);
    assert!(!spring.is_settled());

    let mut spring = spring.with_epsilon(0.001);
    for _ in 0..1_200 {
        spring.advance(1.0 / 240.0);
    }

    assert!(spring.is_settled(), "never settled");
    assert_eq!(spring.value(), DVec2::new(3.0, -4.0));
    assert_eq!(spring.velocity(), DVec2::ZERO);
}

/// The three-component types work the same way, and a spring released at its
/// target with a velocity still comes back — the degenerate case, in 3D.
#[test]
fn a_three_component_glam_spring_returns_to_its_target() {
    let mut spring = Spring::new(Vec3::new(1.0, 2.0, 3.0))
        .with_velocity(Vec3::new(40.0, -25.0, 10.0))
        .with_config(oscillatory());

    for frame in 0..600 {
        spring.advance(1.0 / 60.0);
        assert!(
            spring.value().is_finite(),
            "value became {} on frame {frame}",
            spring.value()
        );
    }

    assert!(spring.is_settled(), "never settled");
    assert_eq!(spring.value(), Vec3::new(1.0, 2.0, 3.0));
}
