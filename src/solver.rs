use crate::spring::SpringDelta;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SpringState<D> {
    pub(crate) displacement: D,
    pub(crate) velocity: D,
}

pub(crate) fn solve<D: SpringDelta>(
    displacement: D,
    velocity: D,
    damping_ratio: f64,
    angular_frequency: f64,
    dt: f64,
) -> SpringState<D> {
    transition(damping_ratio, angular_frequency, dt).apply(displacement, velocity)
}

/// How a state at `t` maps to the state at `t + dt`.
///
/// The closed-form solution is a linear combination of the initial displacement
/// and the initial velocity, so a whole regime collapses into four scalars.
/// Keeping the regime maths in `f64` is what lets the animated type get away
/// with only [`SpringDelta::scale`] and [`SpringDelta::add`]: there is never a
/// need to multiply, divide or take a transcendental function of a `D`.
struct Transition {
    displacement_from_displacement: f64,
    displacement_from_velocity: f64,
    velocity_from_displacement: f64,
    velocity_from_velocity: f64,
}

impl Transition {
    fn apply<D: SpringDelta>(&self, displacement: D, velocity: D) -> SpringState<D> {
        SpringState {
            displacement: displacement
                .scale(self.displacement_from_displacement)
                .add(velocity.scale(self.displacement_from_velocity)),
            velocity: displacement
                .scale(self.velocity_from_displacement)
                .add(velocity.scale(self.velocity_from_velocity)),
        }
    }
}

fn transition(damping_ratio: f64, angular_frequency: f64, dt: f64) -> Transition {
    const CRITICAL_EPSILON: f64 = 1e-4;

    if (damping_ratio - 1.0).abs() < CRITICAL_EPSILON {
        critical(angular_frequency, dt)
    } else if damping_ratio < 1.0 {
        underdamped(damping_ratio, angular_frequency, dt)
    } else {
        overdamped(damping_ratio, angular_frequency, dt)
    }
}

/// x(t) = e^{-ζω₀t}(x₀cos ω_d t + ((v₀ + ζω₀x₀) / ω_d) sin ω_d t)
fn underdamped(damping_ratio: f64, angular_frequency: f64, dt: f64) -> Transition {
    let omega_d = angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();
    let decay = (-damping_ratio * angular_frequency * dt).exp();

    let cos = (omega_d * dt).cos();

    // sin(ω_d·dt) / ω_d turns up in every term; collecting it here is also what
    // makes the ω_d → 0 limit visibly agree with the critical case below.
    let sin_over_omega_d = (omega_d * dt).sin() / omega_d;
    let coupling = damping_ratio * angular_frequency * sin_over_omega_d;

    Transition {
        displacement_from_displacement: decay * (cos + coupling),
        displacement_from_velocity: decay * sin_over_omega_d,
        velocity_from_displacement: -decay
            * angular_frequency
            * angular_frequency
            * sin_over_omega_d,
        velocity_from_velocity: decay * (cos - coupling),
    }
}

/// x(t) = (x₀ + (v₀ + ω₀x₀)t) e^{-ω₀t}
///
/// The ω_d → 0 limit of [`underdamped`]: `cos` → 1 and `sin(ω_d·dt) / ω_d` → dt.
fn critical(angular_frequency: f64, dt: f64) -> Transition {
    let decay = (-angular_frequency * dt).exp();
    let omega_dt = angular_frequency * dt;

    Transition {
        displacement_from_displacement: decay * (1.0 + omega_dt),
        displacement_from_velocity: decay * dt,
        velocity_from_displacement: -decay * angular_frequency * omega_dt,
        velocity_from_velocity: decay * (1.0 - omega_dt),
    }
}

/// x(t) = c₁e^{r₁t} + c₂e^{r₂t}, with two distinct real roots.
fn overdamped(damping_ratio: f64, angular_frequency: f64, dt: f64) -> Transition {
    let root = angular_frequency * (damping_ratio * damping_ratio - 1.0).sqrt();

    let r1 = -damping_ratio * angular_frequency + root;
    let r2 = -damping_ratio * angular_frequency - root;

    let exp1 = (r1 * dt).exp();
    let exp2 = (r2 * dt).exp();
    let span = r1 - r2;

    Transition {
        displacement_from_displacement: (r1 * exp2 - r2 * exp1) / span,
        displacement_from_velocity: (exp1 - exp2) / span,
        // r₁r₂ = ω₀², matching the underdamped row above.
        velocity_from_displacement: r1 * r2 * (exp2 - exp1) / span,
        velocity_from_velocity: (r1 * exp1 - r2 * exp2) / span,
    }
}
