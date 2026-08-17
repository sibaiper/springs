//! [`SpringValue`] and [`SpringDelta`] for glam's vector types.
//!
//! Compiled only when the `glam` feature is on. Implementing *our* traits for
//! *their* types is what the orphan rule permits, so this needs no cooperation
//! from glam and costs nothing when the feature is off.
//!
//! The module is deliberately not called `glam`, so that `glam::Vec2` inside it
//! still refers to the crate rather than to this module.

use crate::spring::{SpringDelta, SpringValue};

/// glam's vectors already have `+`, `-`, scalar `*`, `ZERO` and `length`, so
/// every implementation here is a one-liner forwarding to them.
macro_rules! impl_spring_traits_for_glam_vector {
    ($vector:ty, $scalar:ty) => {
        // `scalar as $scalar` narrows for the f32 vectors, no-ops for the f64 ones.
        #[allow(clippy::unnecessary_cast)]
        impl SpringDelta for $vector {
            fn zero() -> Self {
                <$vector>::ZERO
            }

            fn add(self, other: Self) -> Self {
                self + other
            }

            fn scale(self, scalar: f64) -> Self {
                self * scalar as $scalar
            }

            fn magnitude(self) -> f64 {
                f64::from(self.length())
            }
        }

        impl SpringValue for $vector {
            type Delta = Self;

            fn displacement_from(self, target: Self) -> Self::Delta {
                self - target
            }

            fn add_displacement(self, displacement: Self::Delta) -> Self {
                self + displacement
            }
        }
    };
}

impl_spring_traits_for_glam_vector!(glam::Vec2, f32);
impl_spring_traits_for_glam_vector!(glam::Vec3, f32);
impl_spring_traits_for_glam_vector!(glam::Vec3A, f32);
impl_spring_traits_for_glam_vector!(glam::Vec4, f32);
impl_spring_traits_for_glam_vector!(glam::DVec2, f64);
impl_spring_traits_for_glam_vector!(glam::DVec3, f64);
impl_spring_traits_for_glam_vector!(glam::DVec4, f64);
