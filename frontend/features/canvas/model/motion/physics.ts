const SPRING_STEP = 1 / 240;
const INERTIA_FRICTION = 12;
const INERTIA_START_SPEED = 120;
const INERTIA_STOP_SPEED = 40;
const INERTIA_MAX_SPEED = 1200;

export class Spring {
  value: number;
  target: number;
  private velocity = 0;
  private readonly stiffness: number;
  private readonly damping: number;
  private readonly tolerance: number;

  constructor(value: number, target: number, response: number, ratio: number, tolerance: number) {
    this.value = value;
    this.target = target;
    this.tolerance = tolerance;
    const omega = 2 * Math.PI / response;
    this.stiffness = omega * omega;
    this.damping = 2 * ratio * omega;
  }

  advance(elapsed: number): void {
    const duration = Math.min(1, Math.max(0, elapsed));
    const steps = Math.ceil(duration / SPRING_STEP);
    const step = duration / Math.max(1, steps);
    for (let i = 0; i < steps; i += 1) {
      const acceleration = this.stiffness * (this.target - this.value) - this.damping * this.velocity;
      this.velocity += acceleration * step;
      this.value += this.velocity * step;
    }
    if (this.settled()) {
      this.value = this.target;
      this.velocity = 0;
    }
  }

  settled(): boolean {
    return Math.abs(this.target - this.value) < this.tolerance && Math.abs(this.velocity) < this.tolerance;
  }
}

export class Coast {
  private velocity: [number, number];
  private constructor(velocity: [number, number]) { this.velocity = velocity; }

  static start(velocity: [number, number]): Coast | null {
    const speed = Math.hypot(...velocity);
    if (!Number.isFinite(speed) || speed < INERTIA_START_SPEED) return null;
    const scale = Math.min(INERTIA_MAX_SPEED / speed, 1);
    return new Coast([velocity[0] * scale, velocity[1] * scale]);
  }

  advance(elapsed: number): { next: Coast | null; delta: [number, number] } {
    const [vx, vy] = this.velocity;
    const remaining = Math.log(Math.hypot(vx, vy) / INERTIA_STOP_SPEED) / INERTIA_FRICTION;
    const dt = Math.min(Math.max(elapsed, 0), remaining);
    const decay = Math.exp(-INERTIA_FRICTION * dt);
    const distance = (1 - decay) / INERTIA_FRICTION;
    return {
      next: elapsed < remaining ? new Coast([vx * decay, vy * decay]) : null,
      delta: [vx * distance, vy * distance],
    };
  }
}

