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
        assert!(
            duration.is_finite() && duration > 0.0,
            "spring duration must be finite and greater than zero"
        );

        self.angular_frequency = TAU / duration;
        self
    }

    pub fn bounce(mut self, bounce: f64) -> Self {
        assert!(
            (-1.0..1.0).contains(&bounce),
            "spring bounce must be in -1.0..1.0; a bounce of 1.0 is an undamped \
             spring, which oscillates for ever and never settles"
        );

        self.damping_ratio = 1.0 - bounce;
        self
    }

    pub fn from_duration_bounce(duration: f64, bounce: f64) -> Self {
        assert!(
            duration.is_finite() && duration > 0.0,
            "spring duration must be finite and greater than zero"
        );
        assert!(
            bounce.is_finite() && (-1.0..1.0).contains(&bounce),
            "spring bounce must be in -1.0..1.0; a bounce of 1.0 is an undamped \
             spring, which oscillates for ever and never settles"
        );

        Self {
            damping_ratio: 1.0 - bounce,
            angular_frequency: TAU / duration,
        }
    }

    pub fn from_response_damping(response: f64, damping_ratio: f64) -> Self {
        assert!(
            response.is_finite() && response > 0.0,
            "spring response must be finite and greater than zero"
        );
        assert!(
            damping_ratio.is_finite() && damping_ratio > 0.0,
            "spring damping ratio must be finite and greater than zero; a ratio of \
             zero is an undamped spring, which oscillates for ever and never settles"
        );

        Self {
            damping_ratio,
            angular_frequency: TAU / response,
        }
    }

    pub fn from_physical(mass: f64, stiffness: f64, damping: f64) -> Self {
        assert!(
            mass.is_finite() && mass > 0.0,
            "spring mass must be finite and greater than zero"
        );
        assert!(
            stiffness.is_finite() && stiffness > 0.0,
            "spring stiffness must be finite and greater than zero"
        );
        assert!(
            damping.is_finite() && damping > 0.0,
            "spring damping must be finite and greater than zero; zero damping is \
             an undamped spring, which oscillates for ever and never settles"
        );

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

/// Stiffness and damping are only meaningful next to a mass, so the terms the
/// caller leaves out are derived at `build` time from the ones they set rather
/// than fixed up front. That keeps "unset" meaning the same thing it means for
/// [`SpringConfig::default`] — the default duration and the default bounce —
/// however the other terms are changed.
pub struct PhysicalSpringBuilder {
    mass: f64,
    stiffness: Option<f64>,
    damping: Option<f64>,
}

impl Default for PhysicalSpringBuilder {
    fn default() -> Self {
        Self {
            mass: 1.0,
            stiffness: None,
            damping: None,
        }
    }
}

impl From<PhysicalSpringBuilder> for SpringConfig {
    fn from(value: PhysicalSpringBuilder) -> Self {
        // An unset stiffness is whatever this mass needs to oscillate at the
        // default duration: ω₀ = √(k/m), so k = m·ω₀².
        let stiffness = value.stiffness.unwrap_or_else(|| {
            let angular_frequency = TAU / DEFAULT_DURATION;
            value.mass * angular_frequency * angular_frequency
        });

        // An unset damping is the default bounce at the frequency the mass and
        // stiffness actually produce — not the default one, which the caller
        // may have just moved by setting either of them.
        let damping = value.damping.unwrap_or_else(|| {
            let angular_frequency = (stiffness / value.mass).sqrt();
            2.0 * (1.0 - DEFAULT_BOUNCE) * value.mass * angular_frequency
        });

        SpringConfig::from_physical(value.mass, stiffness, damping)
    }
}

impl PhysicalSpringBuilder {
    pub fn mass(mut self, mass: f64) -> Self {
        self.mass = mass;
        self
    }

    pub fn stiffness(mut self, stiffness: f64) -> Self {
        self.stiffness = Some(stiffness);
        self
    }

    pub fn damping(mut self, damping: f64) -> Self {
        self.damping = Some(damping);
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
