class AnalyticalSpring {
    /**
     * @param {number} dampingRatio - ζ (zeta): < 1 bounces, 1 is critical, > 1 is sluggish.
     * @param {number} angularFrequency - ω0 (omega0): Speed of the spring.
     * @param {number} target - The resting position.
     * @param {number} initialPosition - Starting x.
     */
    constructor(dampingRatio, angularFrequency, target = 0, initialPosition = 0) {
        this.zeta = dampingRatio;
        this.omega0 = angularFrequency;
        this.target = target;

        this.position = initialPosition;
        this.velocity = 0; // Starts at rest
    }

    /**
     * Advances the spring simulation by dt seconds.
     * @param {number} dt - Delta time in seconds (e.g., 0.016 for 60fps)
     */
    step(dt) {
        // If there's no time passed, nothing happens.
        if (dt <= 0) return;

        // Math is done relative to the target (distance from equilibrium)
        const y0 = this.position - this.target; // displacement
        const v0 = this.velocity;
        const zeta = this.zeta;
        const omega0 = this.omega0;

        let y, v;

        // Case A: Critically Damped (ζ ≈ 1)
        // We use a small tolerance because floating point math is rarely exactly 1.0
        if (Math.abs(zeta - 1.0) < 0.0001) {
            const expTerm = Math.exp(-omega0 * dt);
            const c1 = y0;
            const c2 = v0 + omega0 * y0;

            y = (c1 + c2 * dt) * expTerm;
            v = (v0 - omega0 * c2 * dt) * expTerm;
        }

        // Case B: Underdamped (ζ < 1)
        else if (zeta < 1.0) {
            const omegaD = omega0 * Math.sqrt(1.0 - zeta * zeta);
            const expTerm = Math.exp(-zeta * omega0 * dt);

            const cosTerm = Math.cos(omegaD * dt);
            const sinTerm = Math.sin(omegaD * dt);

            const c1 = y0;
            const c2 = (v0 + zeta * omega0 * y0) / omegaD;

            y = expTerm * (c1 * cosTerm + c2 * sinTerm);

            // The velocity derivative simplified
            v = expTerm * (
                v0 * cosTerm -
                (y0 * omegaD + zeta * omega0 * c2) * sinTerm
            );
        }

        // Case C: Overdamped (ζ > 1)
        else {
            const root = omega0 * Math.sqrt(zeta * zeta - 1.0);
            const r1 = -zeta * omega0 + root;
            const r2 = -zeta * omega0 - root;

            const c1 = (v0 - r2 * y0) / (r1 - r2);
            const c2 = (r1 * y0 - v0) / (r1 - r2);

            y = c1 * Math.exp(r1 * dt) + c2 * Math.exp(r2 * dt);
            v = c1 * r1 * Math.exp(r1 * dt) + c2 * r2 * Math.exp(r2 * dt);
        }

        // Convert the relative distance back to absolute world coordinates
        this.position = y + this.target;
        this.velocity = v;
    }
}
