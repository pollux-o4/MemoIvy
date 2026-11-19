import { Spring } from './physics';
import type { Camera } from './geometry';

const CAMERA_RESPONSE = 0.34;

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
    for (const spring of this.springs) spring.advance(elapsed);
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

