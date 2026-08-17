#[derive(Debug, Clone, Copy)]
pub(crate) struct SpringState {
    pub(crate) displacement: f64,
    pub(crate) velocity: f64,
}

pub(crate) fn solve(
    displacement: f64,
    velocity: f64,
    damping_ratio: f64,
    angular_frequency: f64,
    dt: f64,
) -> SpringState {
    const CRITICAL_EPSILON: f64 = 1e-4;

    if (damping_ratio - 1.0).abs() < CRITICAL_EPSILON {
        solve_critical(displacement, velocity, angular_frequency, dt)
    } else if damping_ratio < 1.0 {
        solve_underdamped(displacement, velocity, damping_ratio, angular_frequency, dt)
    } else {
        solve_overdamped(displacement, velocity, damping_ratio, angular_frequency, dt)
    }
}

fn solve_underdamped(
    displacement: f64,
    velocity: f64,
    damping_ratio: f64,
    angular_frequency: f64,
    dt: f64,
) -> SpringState {
    let omega_d = angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();

    let decay = (-damping_ratio * angular_frequency * dt).exp();

    let cos = (omega_d * dt).cos();
    let sin = (omega_d * dt).sin();

    let c1 = displacement;

    let c2 = (velocity + damping_ratio * angular_frequency * displacement) / omega_d;

    let displacement = decay * (c1 * cos + c2 * sin);

    let velocity =
        decay * (velocity * cos - (c1 * omega_d + damping_ratio * angular_frequency * c2) * sin);

    SpringState {
        displacement,
        velocity,
    }
}

fn solve_critical(
    displacement: f64,
    velocity: f64,
    angular_frequency: f64,
    dt: f64,
) -> SpringState {
    let decay = (-angular_frequency * dt).exp();

    let c1 = displacement;
    let c2 = velocity + angular_frequency * displacement;

    let displacement = (c1 + c2 * dt) * decay;

    let velocity = (velocity - angular_frequency * c2 * dt) * decay;

    SpringState {
        displacement,
        velocity,
    }
}

fn solve_overdamped(
    displacement: f64,
    velocity: f64,
    damping_ratio: f64,
    angular_frequency: f64,
    dt: f64,
) -> SpringState {
    let root = angular_frequency * (damping_ratio * damping_ratio - 1.0).sqrt();

    let r1 = -damping_ratio * angular_frequency + root;

    let r2 = -damping_ratio * angular_frequency - root;

    let c1 = (velocity - r2 * displacement) / (r1 - r2);

    let c2 = (r1 * displacement - velocity) / (r1 - r2);

    let exp1 = (r1 * dt).exp();
    let exp2 = (r2 * dt).exp();

    let displacement = c1 * exp1 + c2 * exp2;

    let velocity = c1 * r1 * exp1 + c2 * r2 * exp2;

    SpringState {
        displacement,
        velocity,
    }
}
