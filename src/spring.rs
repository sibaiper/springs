use crate::config::SpringConfig;
use crate::solver::solve;

const DEFAULT_VALUE_EPSILON: f64 = 0.001;
const DEFAULT_VELOCITY_EPSILON: f64 = 0.001;

#[derive(Debug, Clone, Copy)]
pub struct Spring {
    value: f64,
    target: f64,
    velocity: f64,

    config: SpringConfig,
}

impl Spring {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            target: value,
            velocity: 0.0,
            config: SpringConfig::default(),
        }
    }

    pub fn with_config(mut self, config: impl Into<SpringConfig>) -> Self {
        self.config = config.into();
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

    // setters
    pub fn set_target(&mut self, target: f64) {
        self.target = target;
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

    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() < DEFAULT_VALUE_EPSILON
            && self.velocity.abs() < DEFAULT_VELOCITY_EPSILON
    }
}
