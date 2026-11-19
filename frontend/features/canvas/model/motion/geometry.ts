export type Camera = { x: number; y: number; zoom: number };
export type Rect = { x: number; y: number; w: number; h: number };

const MIN_ZOOM = 0.05;
const MAX_ZOOM = 2;
export const ZOOM_STEP = 1.25;
const FIT_PADDING = 72;
const AUTOPAN_EDGE = 36;
const AUTOPAN_SPEED = 900;

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

export function autopan(screen: [number, number], viewport: [number, number], elapsed: number): [number, number] {
  const axis = (position: number, size: number): number => {
    if (position < AUTOPAN_EDGE) return -Math.min((AUTOPAN_EDGE - position) / AUTOPAN_EDGE, 1);
    if (position > size - AUTOPAN_EDGE) return Math.min((position - size + AUTOPAN_EDGE) / AUTOPAN_EDGE, 1);
    return 0;
  };
  const step = AUTOPAN_SPEED * Math.min(elapsed, 0.05);
  return [axis(screen[0], viewport[0]) * step, axis(screen[1], viewport[1]) * step];
}

