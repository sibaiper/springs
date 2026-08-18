# The mathematics behind `springs`

This document derives the state transition used by the solver and describes
exactly what frame-rate independence means when a target can change.

## Equation of motion

Let:

- `y` be the animated value;
- `q` be its target;
- `m` be mass;
- `k` be stiffness;
- `c` be damping.

While the target is fixed, the spring follows

```text
m y'' + c y' + k(y - q) = 0.
```

Using displacement from the target, `x = y - q`, and the standard definitions

```text
ω₀ = √(k / m)
ζ  = c / (2√(km)),
```

the equation becomes

```text
x'' + 2ζω₀x' + ω₀²x = 0.
```

The solver carries the state `(x, v)`, where `v = x' = y'` while `q` is fixed.
Because this differential equation is linear with constant coefficients, its
state after any elapsed time `h` is a linear combination of its initial
displacement and velocity:

```text
[x(h)]   [a(h) b(h)] [x(0)]
[v(h)] = [c(h) d(h)] [v(0)].
```

The four coefficients form the exact state-transition matrix. The
implementation computes them directly for each damping regime.

## Underdamped: `0 < ζ < 1`

Define the damped angular frequency and common decay factor:

```text
ωd = ω₀√(1 - ζ²)
E  = exp(-ζω₀h)
S  = sin(ωd h) / ωd
C  = cos(ωd h).
```

Then

```text
x(h) = E[(C + ζω₀S)x(0) + S v(0)]
v(h) = E[-ω₀²S x(0) + (C - ζω₀S)v(0)].
```

The sine and cosine produce oscillation, while the exponential envelope removes
energy over time.

## Critically damped: `ζ = 1`

At critical damping the characteristic equation has one repeated root. With
`E = exp(-ω₀h)`, the transition is

```text
x(h) = E[(1 + ω₀h)x(0) + h v(0)]
v(h) = E[-ω₀²h x(0) + (1 - ω₀h)v(0)].
```

This is also the limit of the underdamped result as `ωd` approaches zero.

## Overdamped: `ζ > 1`

The characteristic equation has two distinct real roots:

```text
r₁ = -ζω₀ + ω₀√(ζ² - 1)
r₂ = -ζω₀ - ω₀√(ζ² - 1).
```

Let `E₁ = exp(r₁h)`, `E₂ = exp(r₂h)`, and `R = r₁ - r₂`. The transition is

```text
x(h) = [(r₁E₂ - r₂E₁) / R] x(0) + [(E₁ - E₂) / R] v(0)
v(h) = [r₁r₂(E₂ - E₁) / R] x(0) + [(r₁E₁ - r₂E₂) / R] v(0).
```

Both modes decay without oscillating.

## Why this is frame-rate independent

Write the state as `s = [x, v]ᵀ`. The differential equation can be written

```text
s' = A s,

    [    0       1   ]
A = [-ω₀²  -2ζω₀].
```

Its exact solution is

```text
s(t + h) = exp(Ah)s(t).
```

Matrix exponentials have the semigroup property

```text
exp(Ah₂) exp(Ah₁) = exp(A(h₁ + h₂)).
```

Consequently, one update of `1/30` second, two updates of `1/60` second, or any
irregular collection of updates with the same total duration arrive at the same
state. The formulas above are explicit forms of `exp(Ah)`, so `h` is elapsed
time rather than the resolution of a numerical approximation.

In real code, results can differ by floating-point rounding. The library also
snaps sufficiently small displacement and velocity to exactly zero, so the
frame on which a spring reports itself settled can differ near that threshold.
Neither effect accumulates like the integration error of Euler's method.

## Retargeting

Suppose the target changes instantaneously from `q₁` to `q₂`. The animated
value and velocity do not jump:

```text
y⁺ = y⁻
v⁺ = v⁻.
```

Only the displacement coordinate changes:

```text
x⁺ = y - q₂.
```

That `(x⁺, v⁺)` becomes the initial state of another exact segment. Therefore,
if two runs receive the same retarget events at the same physical times, their
results do not depend on how the intervals between those events are divided
into frames.

## Continuously moving targets

For an arbitrary time-varying target `q(t)`, the equation is forced:

```text
y'' + 2ζω₀y' + ω₀²y = ω₀²q(t).
```

Using `z = [y, y']ᵀ`, its exact transition contains an additional integral over
the target's history:

```text
z(t + h) = exp(Ah)z(t)
           + ∫₀ʰ exp(A(h - τ)) B q(t + τ) dτ,

B = [0, ω₀²]ᵀ.
```

That integral cannot be evaluated from only the target's value at the end of a
rendered frame. The target must have a known model—constant, linear, polynomial,
or another integrable function—or its history must be approximated numerically.

`springs` uses a piecewise-constant model: each call to `set_target` changes the
target immediately, and that target is held until the next call. This is also
called a zero-order hold. Every held interval is solved exactly, but sampling a
continuously moving source at different render rates produces different input
histories. Any resulting difference comes from input sampling, not from
time-stepping error in the spring transition.

## Configuration mappings

The animation-oriented configuration maps to the normalized equation as

```text
ω₀ = 2π / duration
ζ  = 1 - bounce.
```

Therefore positive bounce is underdamped, zero bounce is critically damped, and
negative bounce is overdamped. The physical configuration uses the definitions
of `ω₀` and `ζ` given at the start of this document.

For vectors and arrays, the same scalar transition is applied independently to
every component. The solver only requires displacement scaling and addition;
the magnitude operation is used solely to decide when the spring has settled.
