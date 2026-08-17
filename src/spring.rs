use crate::config::SpringConfig;
use crate::solver::solve;

const DEFAULT_EPSILON: f64 = 0.001;

#[derive(Debug, Clone, Copy)]
pub struct Spring {
    value: f64,
    target: f64,
    velocity: f64,

    epsilon: f64,
    config: SpringConfig,
}

impl Spring {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            target: value,
            velocity: 0.0,
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

    pub fn with_target(mut self, target: f64) -> Self {
        self.target = target;
        self
    }
    pub fn with_velocity(mut self, velocity: f64) -> Self {
        self.velocity = velocity;
        self
    }

    // getters
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn velocity(&self) -> f64 {
        self.velocity
    }

    pub fn target(&self) -> f64 {
        self.target
    }

    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    // setters
    pub fn set_target(&mut self, target: f64) {
        self.target = target;
    }

    pub fn set_epsilon(&mut self, epsilon: f64) {
        assert!(
            epsilon.is_finite() && epsilon > 0.0,
            "spring epsilon must be finite and greater than zero"
        );

        self.epsilon = epsilon;
    }
    pub fn set_velocity(&mut self, velocity: f64) {
        self.velocity = velocity;
    }

    pub fn add_velocity(&mut self, velocity: f64) {
        self.velocity += velocity;
    }

    pub fn snap_to(&mut self, value: f64) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    pub fn advance(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        let displacement = self.value - self.target;

        let state = solve(
            displacement,
            self.velocity,
            self.config.damping_ratio(),
            self.config.angular_frequency(),
            dt,
        );

        self.value = self.target + state.displacement;
        self.velocity = state.velocity;

        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
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
        (self.value - self.target).abs() < self.epsilon
            && self.velocity.abs() < self.epsilon * self.config.angular_frequency()
    }
}
