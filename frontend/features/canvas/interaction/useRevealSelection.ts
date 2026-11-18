import { useCallback, useEffect, useRef, useState } from 'react';
import type { RefObject } from 'react';
import type { Camera } from '../model/motion';

type Rect = { x: number; y: number; w: number; h: number };

const MARGIN = 12;
const INSPECTOR_WIDTH = 284;

/**
 * 선택한 카드가 인스펙터에 가려지거나 화면 밖이면 보이는 곳으로 카메라를 옮긴다.
 * 사용자가 직접 누른 카드는 이미 보고 있으므로 옮기지 않는다: 호출자가 reveal()을 요청할 때만(새로 만든 카드,
 * 인스펙터의 연결 카드 선택 등) 한 번 실행한다. 카메라·위치 변화에는 반응하지 않아 이동 중 끌려가지 않는다.
 */
export function useRevealSelection(host: RefObject<HTMLDivElement | null>, rect: Rect | undefined, camera: Camera, animateTo: (next: Camera) => void) {
  const [request, setRequest] = useState(0);
  const latest = useRef({ rect, camera, animateTo });
  latest.current = { rect, camera, animateTo };
  useEffect(() => {
    const board = host.current;
    const { rect: target, camera: view, animateTo: move } = latest.current;
    if (!request || !board || !target) return;
    const width = board.clientWidth; const height = board.clientHeight;
    const visibleWidth = Math.max(width - INSPECTOR_WIDTH, width * .35);
    const left = (target.x - view.x) * view.zoom; const top = (target.y - view.y) * view.zoom;
    const right = left + target.w * view.zoom; const bottom = top + target.h * view.zoom;
    if (left >= MARGIN && top >= MARGIN && right <= visibleWidth - MARGIN && bottom <= height - MARGIN) return;
    const centerX = target.x + target.w / 2; const centerY = target.y + target.h / 2;
    move({ x: centerX - visibleWidth / view.zoom / 2, y: centerY - height / view.zoom / 2, zoom: view.zoom });
  }, [request, host]);
  return useCallback(() => setRequest((value) => value + 1), []);
}
