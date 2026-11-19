import { useCallback, useEffect, useRef, useState } from 'react';
import type { RefObject } from 'react';
import type { Camera } from '../../model/motion';
import { type Rect, revealCamera, visibleArea } from '../../model/reveal';

/** 보드 위에 겹쳐 떠 카드를 가릴 수 있는 패널. */
const OVERLAY_SELECTOR = '.canvas-inspector';

const toRect = (r: DOMRect): Rect => ({ x: r.left, y: r.top, w: r.width, h: r.height });

/**
 * 선택한 카드가 겹친 패널(인스펙터)에 가려지거나 화면 밖이면 보이는 곳까지 카메라를 최소한만 옮긴다.
 * - 단일 선택이 바뀌면 조작(끌기·잇기 등)이 끝난 뒤 한 번 확인한다. 직접 누른 카드도 인스펙터가 새로 열리며 가릴 수 있다.
 * - reveal(): 선택이 그대로여도 다시 확인한다(새로 만든 카드 등).
 * 가림 영역은 실행 시점의 실제 패널 크기로 잰다. 카메라·위치 변화에는 반응하지 않아 이동 중 끌려가지 않는다.
 */
export function useRevealSelection(host: RefObject<HTMLDivElement | null>, target: { id: string; rect: Rect } | undefined,
  getCamera: () => Camera, animateTo: (next: Camera) => void, idle: boolean) {
  const [request, setRequest] = useState(0);
  const pending = useRef(false);
  const latest = useRef({ target, getCamera, animateTo });
  latest.current = { target, getCamera, animateTo };
  // biome-ignore lint/correctness/useExhaustiveDependencies: 선택 id 변화·요청이 확인 계기다.
  useEffect(() => { if (target) pending.current = true; }, [target?.id, request]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: 나머지는 실행 시점의 최신 값을 읽는다.
  useEffect(() => {
    const board = host.current;
    const { target: current, getCamera: readView, animateTo: move } = latest.current;
    if (!pending.current || !idle || !board || !current) return;
    pending.current = false;
    const overlays = [...board.parentElement?.querySelectorAll(OVERLAY_SELECTOR) ?? []].map((el) => toRect(el.getBoundingClientRect()));
    const next = revealCamera(current.rect, readView(), visibleArea(toRect(board.getBoundingClientRect()), overlays));
    if (next) move(next);
  }, [target?.id, request, idle, host]);
  return useCallback(() => setRequest((value) => value + 1), []);
}
