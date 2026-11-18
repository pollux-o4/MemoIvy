export type Camera = { x: number; y: number; zoom: number };
export type Rect = { x: number; y: number; w: number; h: number };

const MIN_ZOOM = 0.05;
const MAX_ZOOM = 2;
export const ZOOM_STEP = 1.25;
const FIT_PADDING = 72;
const TRAVEL_SECONDS = 0.65;
const SIGNAL_SECONDS = 0.95;
const LINK_SECONDS = 0.28;
const CAMERA_RESPONSE = 0.34;
const CARD_RESPONSE = 0.36;
const CARD_DAMPING = 0.84;

const SPRING_STEP = 1 / 240;
const INERTIA_FRICTION = 12;
const INERTIA_START_SPEED = 120;
const INERTIA_STOP_SPEED = 40;
const INERTIA_MAX_SPEED = 1200;
const AUTOPAN_EDGE = 36;
const AUTOPAN_SPEED = 900;

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

export function fitting(bounds: Rect, viewport: [number, number], padding = FIT_PADDING): Camera {
  const width = Math.max(viewport[0] - padding * 2, 1);
  const height = Math.max(viewport[1] - padding * 2, 1);
  const zoom = Math.min(width / Math.max(bounds.w, 1), height / Math.max(bounds.h, 1), 1);
  const clamped = Math.max(MIN_ZOOM, Math.min(zoom, 1));
  return centeredOn(bounds, viewport, clamped);
}

export function centeredOn(bounds: Rect, viewport: [number, number], zoom: number): Camera {
  return {
    x: bounds.x + bounds.w / 2 - viewport[0] / zoom / 2,
    y: bounds.y + bounds.h / 2 - viewport[1] / zoom / 2,
    zoom,
  };
}

function shows(camera: Camera, rect: Rect, viewport: [number, number], padding: number): boolean {
  const left = (rect.x - camera.x) * camera.zoom;
  const top = (rect.y - camera.y) * camera.zoom;
  const right = (rect.x + rect.w - camera.x) * camera.zoom;
  const bottom = (rect.y + rect.h - camera.y) * camera.zoom;
  return left >= padding && top >= padding && right <= viewport[0] - padding && bottom <= viewport[1] - padding;
}

export function bounds(rects: Rect[]): Rect | null {
  if (rects.length === 0) return null;
  let { x: left, y: top, w, h } = rects[0];
  let right = left + w;
  let bottom = top + h;
  for (const rect of rects.slice(1)) {
    left = Math.min(left, rect.x);
    top = Math.min(top, rect.y);
    right = Math.max(right, rect.x + rect.w);
    bottom = Math.max(bottom, rect.y + rect.h);
  }
  return { x: left, y: top, w: right - left, h: bottom - top };
}

export function firstLayout(rects: Rect[], selected: Rect[], viewport: [number, number]): Camera | null {
  const area = bounds(rects);
  if (!area) return null;
  let camera = fitting(area, viewport);
  const focus = bounds(selected);
  if (focus && !shows(camera, focus, viewport, 24)) camera = centeredOn(focus, viewport, 1);
  return camera;
}

/** 화면 거리만큼 이동한다(GTK Camera::panned). */
export function panned(camera: Camera, [dx, dy]: [number, number]): Camera {
  return { ...camera, x: camera.x + dx / camera.zoom, y: camera.y + dy / camera.zoom };
}

/** 연속 휠로 이동 목표가 현재 화면보다 너무 앞서 나가지 않게 화면 기준 maxLead(px) 안으로 줄인다. */
export function limitLead(current: Camera, target: Camera, maxLead: number): Camera {
  const dx = (target.x - current.x) * current.zoom; const dy = (target.y - current.y) * current.zoom;
  const lead = Math.hypot(dx, dy);
  if (lead <= maxLead) return target;
  const ratio = maxLead / lead;
  return { ...target, x: current.x + (target.x - current.x) * ratio, y: current.y + (target.y - current.y) * ratio };
}

export function zoomAt(camera: Camera, screen: [number, number], zoom: number): Camera {
  const clamped = Math.max(MIN_ZOOM, Math.min(zoom, MAX_ZOOM));
  const worldX = screen[0] / camera.zoom + camera.x;
  const worldY = screen[1] / camera.zoom + camera.y;
  return { x: worldX - screen[0] / clamped, y: worldY - screen[1] / clamped, zoom: clamped };
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

export class CameraMotion {
  current: Camera;
  private springs: [Spring, Spring, Spring] | null = null;
  private readonly viewport: [number, number];

  constructor(viewport: [number, number]) {
    this.viewport = viewport;
    this.current = { x: 0, y: 0, zoom: 1 };
  }

  to(target: Camera, animated: boolean): void {
    if (!animated) {
      this.current = { ...target };
      this.springs = null;
      return;
    }
    const destination = cameraValues(target, this.viewport);
    if (this.springs) {
      this.springs.forEach((spring, i) => { spring.target = destination[i]; });
      return;
    }
    const origin = cameraValues(this.current, this.viewport);
    this.springs = destination.map((value, i) =>
      new Spring(origin[i], value, CAMERA_RESPONSE, 1, i === 0 ? 0.0004 : 0.08),
    ) as [Spring, Spring, Spring];
  }

  tick(elapsed: number): Camera {
    if (!this.springs) return this.current;
    this.springs.forEach((spring) => spring.advance(elapsed));
    const [logZoom, centerX, centerY] = this.springs.map((spring) => spring.value);
    const zoom = Math.exp(logZoom);
    this.current = {
      x: centerX - this.viewport[0] / zoom / 2,
      y: centerY - this.viewport[1] / zoom / 2,
      zoom,
    };
    if (this.springs.every((spring) => spring.settled())) this.springs = null;
    return this.current;
  }

  get settled(): boolean { return this.springs === null; }

  /** 진행 중이면 도착 예정 카메라, 아니면 현재 카메라. 연속 입력은 여기서 이어 붙인다(GTK motion_target). */
  get target(): Camera {
    if (!this.springs) return this.current;
    const zoom = Math.exp(this.springs[0].target);
    return { x: this.springs[1].target - this.viewport[0] / zoom / 2, y: this.springs[2].target - this.viewport[1] / zoom / 2, zoom };
  }
}

function cameraValues(camera: Camera, viewport: [number, number]): [number, number, number] {
  return [camera.zoom === 0 ? -Infinity : Math.log(camera.zoom), camera.x + viewport[0] / camera.zoom / 2, camera.y + viewport[1] / camera.zoom / 2];
}

export function autopan(screen: [number, number], viewport: [number, number], elapsed: number): [number, number] {
  const axis = (position: number, size: number): number => {
    if (position < AUTOPAN_EDGE) return -Math.min((AUTOPAN_EDGE - position) / AUTOPAN_EDGE, 1);
    if (position > size - AUTOPAN_EDGE) return Math.min((position - size + AUTOPAN_EDGE) / AUTOPAN_EDGE, 1);
    return 0;
  };
  const step = AUTOPAN_SPEED * Math.min(elapsed, 0.05);
  return [axis(screen[0], viewport[0]) * step, axis(screen[1], viewport[1]) * step];
}

function ease(t: number): number {
  const value = Math.max(0, Math.min(t, 1));
  return value * value * (3 - 2 * value);
}

export function pulseSample(elapsed: number, animated: boolean): { travel: number; alpha: number } {
  if (!animated) return { travel: 1, alpha: 1 };
  return {
    travel: ease(elapsed / TRAVEL_SECONDS),
    alpha: 1 - Math.max(0, Math.min((elapsed - TRAVEL_SECONDS) / (SIGNAL_SECONDS - TRAVEL_SECONDS), 1)),
  };
}

export function linkProgress(elapsed: number): number {
  return ease(elapsed / LINK_SECONDS);
}
