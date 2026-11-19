import { type RefObject, useRef } from 'react';
import type { CanvasPoint } from '../model/align';
import type { Camera } from '../model/motion';

/**
 * 포인터 좌표 변환: 화면(client) → 보드(screen) → 캔버스(world).
 * 보드 원점은 누를 때만 잰다(fresh). 끌기 중 매 이동마다 getBoundingClientRect를 부르면 강제 레이아웃으로 프레임당 수 ms가 든다.
 */
export function useBoardPoint(host: RefObject<HTMLDivElement | null>, camera: Camera) {
  const origin = useRef<{ left: number; top: number } | null>(null);
  const measure = () => { const rect = host.current?.getBoundingClientRect(); origin.current = { left: rect?.left ?? 0, top: rect?.top ?? 0 }; return origin.current; };
  const screenPoint = (event: { clientX: number; clientY: number }, fresh = false): CanvasPoint => {
    const at = fresh || !origin.current ? measure() : origin.current;
    return { x: event.clientX - at.left, y: event.clientY - at.top };
  };
  const worldPoint = (screen: CanvasPoint): CanvasPoint => ({ x: screen.x / camera.zoom + camera.x, y: screen.y / camera.zoom + camera.y });
  return { screenPoint, worldPoint };
}
