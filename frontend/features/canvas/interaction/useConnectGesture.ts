import { type PointerEvent as ReactPointerEvent, type RefObject, useRef, useState } from 'react';
import type { Note } from '../../../shared/engine_contract';
import type { CanvasPoint } from '../model/align';
import { topCardAt } from '../model/hit';
import type { CanvasMode } from './useCanvasSelection';
import type { useBoardPoint } from './useBoardPoint';

/** 이 거리(화면 px) 넘게 움직여야 끌기로 본다. 그 아래는 누르기. */
export const DRAG_SLOP_PX = 3;

export type ConnectPreviewState = { to: CanvasPoint; target?: string };
export type ConnectOutcome = {
  onConnect: (from: string, to: string) => void;
  /** 빈 곳에 놓았다: 그 자리(캔버스 좌표)에 새 가지. */
  onConnectEmpty?: (from: string, at: CanvasPoint) => void;
};

/**
 * 잇기 제스처. 포인터 상태 기계의 connect 모드를 맡는다.
 * - 끌기: 손잡이 누르기 → 끌며 미리보기 → 카드(연결)·빈 곳(새 가지)에 놓기.
 * - 고르기: 끌지 않고 뗐으면 바로 만들지 않고 고르기 상태로 남는다. 미리보기 선이 포인터를 따라가고,
 *   다음 누르기가 카드면 연결, 빈 곳이면 그 자리에 새 가지, 출발 카드·Esc면 취소.
 */
export function useConnectGesture(o: ConnectOutcome & {
  host: RefObject<HTMLDivElement | null>; point: ReturnType<typeof useBoardPoint>;
  connectFrom: string | null; setConnectFrom: (id: string | null) => void; setMode: (mode: CanvasMode) => void;
  notes: Note[]; positions: Record<string, CanvasPoint>;
}) {
  const [preview, setPreview] = useState<ConnectPreviewState | null>(null);
  const down = useRef<{ point: CanvasPoint; moved: boolean } | null>(null);
  const [picking, setPicking] = useState(false);
  const reset = () => { down.current = null; setPreview(null); setPicking(false); };
  const track = (event: ReactPointerEvent) => {
    const world = o.point.worldPoint(o.point.screenPoint(event));
    setPreview({ to: world, target: topCardAt(world, o.notes, o.positions, (note) => note.id !== o.connectFrom)?.id });
  };
  const finish = (from: string, target: string | undefined, event: ReactPointerEvent) => {
    if (target && target !== from) o.onConnect(from, target);
    else if (!target) o.onConnectEmpty?.(from, o.point.worldPoint(o.point.screenPoint(event)));
  };
  const close = () => { reset(); o.setConnectFrom(null); o.setMode('idle'); };
  return {
    preview: down.current?.moved || picking ? preview : null,
    picking,
    reset,
    begin: (id: string, event?: ReactPointerEvent) => {
      o.setConnectFrom(id); o.setMode('connect');
      if (!event) return;
      down.current = { point: o.point.screenPoint(event, true), moved: false };
      o.host.current?.setPointerCapture(event.pointerId);
    },
    move: (event: ReactPointerEvent) => {
      if (!o.connectFrom) return;
      if (picking) { track(event); return; }
      const start = down.current;
      if (!start) return;
      const screen = o.point.screenPoint(event);
      if (!start.moved && Math.hypot(screen.x - start.point.x, screen.y - start.point.y) <= DRAG_SLOP_PX) return;
      start.moved = true;
      track(event);
    },
    end: (target: string | undefined, event: ReactPointerEvent) => {
      const from = o.connectFrom;
      if (from && down.current && !down.current.moved && !picking) { down.current = null; setPicking(true); track(event); return; }
      if (from) finish(from, target, event);
      close();
    },
    /** 고르기 상태의 다음 누르기. 카드·손잡이의 자체 동작(끌기·선택)보다 먼저 가로챈다. */
    pick: (target: string | undefined, event: ReactPointerEvent) => {
      const from = o.connectFrom;
      if (from && target !== from) finish(from, target, event);
      close();
    },
  };
}
