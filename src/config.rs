use std::f64::consts::TAU;

const DEFAULT_DURATION: f64 = 0.3;
const DEFAULT_BOUNCE: f64 = 0.0;

#[derive(Debug, Clone, Copy)]
pub struct SpringConfig {
    damping_ratio: f64,
    angular_frequency: f64,
}

impl SpringConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn duration(mut self, duration: f64) -> Self {
        assert!(duration > 0.0);

        self.angular_frequency = TAU / duration;
        self
    }

    pub fn bounce(mut self, bounce: f64) -> Self {
        assert!((-1.0..=1.0).contains(&bounce));

        self.damping_ratio = 1.0 - bounce;
        self
    }

    pub fn from_duration_bounce(duration: f64, bounce: f64) -> Self {
        assert!(
            duration.is_finite() && duration > 0.0,
            "spring duration must be greater than zero"
        );
        assert!(
            bounce.is_finite() && (-1.0..=1.0).contains(&bounce),
            "spring bounce must be between -1.0 and 1.0"
        );

        Self {
            damping_ratio: 1.0 - bounce,
            angular_frequency: TAU / duration,
        }
    }

    pub fn from_response_damping(response: f64, damping_ratio: f64) -> Self {
        assert!(response > 0.0);
        assert!(damping_ratio >= 0.0);

        Self {
            damping_ratio,
            angular_frequency: TAU / response,
        }
    }

    pub fn from_physical(mass: f64, stiffness: f64, damping: f64) -> Self {
        assert!(mass.is_finite() && mass > 0.0);
        assert!(stiffness.is_finite() && stiffness > 0.0);
        assert!(damping.is_finite() && damping >= 0.0);

        Self {
            angular_frequency: (stiffness / mass).sqrt(),

            damping_ratio: damping / (2.0 * (stiffness * mass).sqrt()),
        }
    }

    pub fn physical() -> PhysicalSpringBuilder {
        PhysicalSpringBuilder::default()
    }

    pub fn responsive() -> ResponsiveSpringBuilder {
        ResponsiveSpringBuilder::default()
    }

    // getters
    pub fn damping_ratio(self) -> f64 {
        self.damping_ratio
    }
    // getters
    pub fn angular_frequency(self) -> f64 {
        self.angular_frequency
    }
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self::from_duration_bounce(DEFAULT_DURATION, DEFAULT_BOUNCE)
    }
}

pub struct PhysicalSpringBuilder {
    mass: f64,
    stiffness: f64,
    damping: f64,
}

impl Default for PhysicalSpringBuilder {
    fn default() -> Self {
        let mass = 1.0;
        let damping_ratio = 1.0 - DEFAULT_BOUNCE;
        let angular_frequency = TAU / DEFAULT_DURATION;

        let stiffness = mass * angular_frequency * angular_frequency;

        let damping = 2.0 * damping_ratio * mass * angular_frequency;

        Self {
            mass,
            stiffness,
            damping,
        }
    }
}

impl From<PhysicalSpringBuilder> for SpringConfig {
    fn from(value: PhysicalSpringBuilder) -> Self {
        SpringConfig::from_physical(value.mass, value.stiffness, value.damping)
    }
}

impl PhysicalSpringBuilder {
    pub fn mass(mut self, mass: f64) -> Self {
        self.mass = mass;
        self
    }

    pub fn stiffness(mut self, stiffness: f64) -> Self {
        self.stiffness = stiffness;
        self
    }

    pub fn damping(mut self, damping: f64) -> Self {
        self.damping = damping;
        self
    }

    pub fn build(self) -> SpringConfig {
        self.into()
    }
}

pub struct ResponsiveSpringBuilder {
    response: f64,
    damping_ratio: f64,
}

impl Default for ResponsiveSpringBuilder {
    fn default() -> Self {
        Self {
            response: DEFAULT_DURATION,
            damping_ratio: 1.0 - DEFAULT_BOUNCE,
        }
    }
}

impl From<ResponsiveSpringBuilder> for SpringConfig {
    fn from(value: ResponsiveSpringBuilder) -> Self {
        SpringConfig::from_response_damping(value.response, value.damping_ratio)
    }
}

impl ResponsiveSpringBuilder {
    pub fn response(mut self, response: f64) -> Self {
        self.response = response;
        self
    }

    pub fn damping(mut self, damping_ratio: f64) -> Self {
        self.damping_ratio = damping_ratio;
        self
    }

    pub fn build(self) -> SpringConfig {
        self.into()
    }
}
