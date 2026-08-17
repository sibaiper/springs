//! Ready-made [`SpringValue`] and [`SpringDelta`] implementations.

use std::f64::consts::TAU;

use crate::spring::{SpringDelta, SpringValue};

/// Implements both traits for a float and, in one go, for arrays of it of
/// *every* length — `[f64; 2]`, `[f64; 3]`, `[f64; 4]` and so on all come from
/// the single const-generic impl below.
macro_rules! impl_spring_traits_for_float {
    ($float:ty) => {
        // `scalar as $float` is a real narrowing for f32 and a no-op for f64.
        #[allow(clippy::unnecessary_cast)]
        impl SpringDelta for $float {
            fn zero() -> Self {
                0.0
            }

            fn add(self, other: Self) -> Self {
                self + other
            }

            fn scale(self, scalar: f64) -> Self {
                self * scalar as $float
            }

            fn magnitude(self) -> f64 {
                f64::from(self.abs())
            }
        }

        impl SpringValue for $float {
            type Delta = Self;

            fn displacement_from(self, target: Self) -> Self::Delta {
                self - target
            }

            fn add_displacement(self, displacement: Self::Delta) -> Self {
                self + displacement
            }
        }

        #[allow(clippy::unnecessary_cast)]
        impl<const N: usize> SpringDelta for [$float; N] {
            fn zero() -> Self {
                [0.0; N]
            }

            fn add(self, other: Self) -> Self {
                std::array::from_fn(|index| self[index] + other[index])
            }

            fn scale(self, scalar: f64) -> Self {
                std::array::from_fn(|index| self[index] * scalar as $float)
            }

            /// Euclidean length, so a spring settles on the distance it still
            /// has to travel rather than on any one component.
            fn magnitude(self) -> f64 {
                self.iter()
                    .map(|component| f64::from(*component).powi(2))
                    .sum::<f64>()
                    .sqrt()
            }
        }

        impl<const N: usize> SpringValue for [$float; N] {
            type Delta = Self;

            fn displacement_from(self, target: Self) -> Self::Delta {
                std::array::from_fn(|index| self[index] - target[index])
            }

            fn add_displacement(self, displacement: Self::Delta) -> Self {
                self.add(displacement)
            }
        }
    };
}

impl_spring_traits_for_float!(f32);
impl_spring_traits_for_float!(f64);

/// An angle that always animates the short way round.
///
/// Springing from 359° to 2° travels +3°, not −357°: the displacement between
/// two angles is wrapped into (−π, π], so the spring never notices the longer
/// arc exists. That wrapping is the whole of the angle-specific behaviour, and
/// it lives in [`SpringValue::displacement_from`] — the delta is a plain `f64`
/// of radians, reusing the scalar implementation above.
///
/// Every `Angle` is normalised to `[0, τ)` on construction, so [`Angle::radians`]
/// and [`Angle::degrees`] always report a value in range. Winding — "spin three
/// times and settle" — is deliberately not representable; animate a plain `f64`
/// of turns if you want that.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Angle {
    radians: f64,
}

impl Angle {
    /// Wraps `radians` into `[0, τ)`.
    pub fn from_radians(radians: f64) -> Self {
        assert!(radians.is_finite(), "an angle must be finite");

        Self {
            radians: radians.rem_euclid(TAU),
        }
    }

    /// Wraps `degrees` into `[0, 360)`.
    pub fn from_degrees(degrees: f64) -> Self {
        Self::from_radians(degrees.to_radians())
    }

    pub fn radians(self) -> f64 {
        self.radians
    }

    pub fn degrees(self) -> f64 {
        self.radians.to_degrees()
    }
}

impl SpringValue for Angle {
    /// An angular displacement is a signed scalar, in radians.
    type Delta = f64;

    fn displacement_from(self, target: Self) -> Self::Delta {
        // Wrap into (-π, π]: whichever way round is shorter.
        let raw = self.radians - target.radians;

        raw - TAU * (raw / TAU).round()
    }

    fn add_displacement(self, displacement: Self::Delta) -> Self {
        Self::from_radians(self.radians + displacement)
    }
}
