use crate::config::SpringConfig;
use crate::solver::solve;

const DEFAULT_EPSILON: f64 = 0.001;

/// A value a spring can animate.
///
/// The value itself only needs to know how to subtract to a [`SpringDelta`] and
/// how to add one back. Everything else happens in the delta, which is where
/// the physics lives — that split is what lets a spring animate a type that is
/// not itself a vector space, like a colour or a point on a curve.
pub trait SpringValue: Copy {
    /// The difference between two values. For a scalar this is the same type;
    /// for a point it would be the corresponding vector.
    type Delta: SpringDelta;

    /// The displacement of `self` from `target`, i.e. `self - target`.
    fn displacement_from(self, target: Self) -> Self::Delta;

    /// `self` displaced by `displacement`, i.e. `self + displacement`.
    fn add_displacement(self, displacement: Self::Delta) -> Self;
}

/// The vector space the solver integrates in.
///
/// The closed-form solution is a linear combination of the initial displacement
/// and velocity, so scaling by an `f64` and adding are the only operations it
/// needs — there is never a delta multiplied by another delta. `magnitude` is
/// separate: it is the one place a delta collapses to a scalar, so the spring
/// can compare it against its epsilon.
pub trait SpringDelta: Copy {
    fn zero() -> Self;

    fn add(self, other: Self) -> Self;

    fn scale(self, scalar: f64) -> Self;

    /// The length of this delta, as a scalar.
    fn magnitude(self) -> f64;
}

impl SpringDelta for f64 {
    fn zero() -> Self {
        0.0
    }

    fn add(self, other: Self) -> Self {
        self + other
    }

    fn scale(self, scalar: f64) -> Self {
        self * scalar
    }

    fn magnitude(self) -> f64 {
        self.abs()
    }
}

impl SpringValue for f64 {
    type Delta = f64;

    fn displacement_from(self, target: Self) -> Self::Delta {
        self - target
    }

    fn add_displacement(self, displacement: Self::Delta) -> Self {
        self + displacement
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Spring<T>
where
    T: SpringValue,
{
    value: T,
    target: T,
    velocity: T::Delta,

    epsilon: f64,
    config: SpringConfig,
}

impl<T> Spring<T>
where
    T: SpringValue,
{
    pub fn new(value: T) -> Self {
        Self {
            value,
            target: value,
            velocity: T::Delta::zero(),
            epsilon: DEFAULT_EPSILON,
            config: SpringConfig::default(),
        }
    }

    pub fn with_config(mut self, config: impl Into<SpringConfig>) -> Self {
        self.config = config.into();
        self
    }

    /// Sets how close to the target counts as arrived, in the same units as the
    /// animated value. A spring driving pixels wants a coarser epsilon than one
    /// driving an opacity: the default of 0.001 is a thousandth of an opacity
    /// but a thousandth of a pixel.
    pub fn with_epsilon(mut self, epsilon: f64) -> Self {
        self.set_epsilon(epsilon);
        self
    }

    pub fn with_target(mut self, target: T) -> Self {
        self.target = target;
        self
    }
    pub fn with_velocity(mut self, velocity: T::Delta) -> Self {
        self.velocity = velocity;
        self
    }

    // getters
    pub fn value(&self) -> T {
        self.value
    }

    pub fn velocity(&self) -> T::Delta {
        self.velocity
    }

    pub fn target(&self) -> T {
        self.target
    }

    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    // setters
    pub fn set_target(&mut self, target: T) {
        self.target = target;
    }

    pub fn set_epsilon(&mut self, epsilon: f64) {
        assert!(
            epsilon.is_finite() && epsilon > 0.0,
            "spring epsilon must be finite and greater than zero"
        );

        self.epsilon = epsilon;
    }
    pub fn set_velocity(&mut self, velocity: T::Delta) {
        self.velocity = velocity;
    }

    pub fn add_velocity(&mut self, velocity: T::Delta) {
        self.velocity = self.velocity.add(velocity);
    }

    pub fn snap_to(&mut self, value: T) {
        self.value = value;
        self.target = value;
        self.velocity = T::Delta::zero();
    }

    pub fn advance(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        let displacement = self.value.displacement_from(self.target);

        let state = solve(
            displacement,
            self.velocity,
            self.config.damping_ratio(),
            self.config.angular_frequency(),
            dt,
        );

        self.value = self.target.add_displacement(state.displacement);
        self.velocity = state.velocity;

        if self.is_settled() {
            self.value = self.target;
            self.velocity = T::Delta::zero();
        }
    }

    /// Whether the spring has arrived and stopped.
    ///
    /// The velocity threshold is `epsilon · ω₀` rather than a second, unrelated
    /// constant: the two have different units, and 1/ω₀ is the spring's own
    /// timescale, so `epsilon · ω₀` is the speed at which it still has about
    /// `epsilon` of travel left. Pairing them this way makes both conditions
    /// trip at the same moment instead of leaving the velocity one to dominate.
    pub fn is_settled(&self) -> bool {
        self.value.displacement_from(self.target).magnitude() < self.epsilon
            && self.velocity.magnitude() < self.epsilon * self.config.angular_frequency()
    }
}
