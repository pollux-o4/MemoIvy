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
  /** 끌지 않고 눌렀다 뗐다(보드가 포인터를 잡아 버튼 click이 오지 않으므로 여기서 받는다). */
  onConnectTap?: (from: string) => void;
};

/** 잇기 제스처: 손잡이 누르기 → 끌며 미리보기 → 카드·빈 곳에 놓기 또는 누르기만. 포인터 상태 기계의 connect 모드를 맡는다. */
export function useConnectGesture(o: ConnectOutcome & {
  host: RefObject<HTMLDivElement | null>; point: ReturnType<typeof useBoardPoint>;
  connectFrom: string | null; setConnectFrom: (id: string | null) => void; setMode: (mode: CanvasMode) => void;
  notes: Note[]; positions: Record<string, CanvasPoint>;
}) {
  const [preview, setPreview] = useState<ConnectPreviewState | null>(null);
  const down = useRef<{ point: CanvasPoint; moved: boolean } | null>(null);
  const reset = () => { down.current = null; setPreview(null); };
  return {
    preview: down.current?.moved ? preview : null,
    reset,
    begin: (id: string, event?: ReactPointerEvent) => {
      o.setConnectFrom(id); o.setMode('connect');
      if (!event) return;
      down.current = { point: o.point.screenPoint(event, true), moved: false };
      o.host.current?.setPointerCapture(event.pointerId);
    },
    move: (event: ReactPointerEvent) => {
      const start = down.current;
      if (!start || !o.connectFrom) return;
      const screen = o.point.screenPoint(event);
      if (!start.moved && Math.hypot(screen.x - start.point.x, screen.y - start.point.y) <= DRAG_SLOP_PX) return;
      start.moved = true;
      const world = o.point.worldPoint(screen);
      setPreview({ to: world, target: topCardAt(world, o.notes, o.positions, (note) => note.id !== o.connectFrom)?.id });
    },
    end: (target: string | undefined, event: ReactPointerEvent) => {
      const from = o.connectFrom; const moved = down.current?.moved; const pressed = Boolean(down.current);
      if (from && target && target !== from) o.onConnect(from, target);
      else if (from && !target && moved) o.onConnectEmpty?.(from, o.point.worldPoint(o.point.screenPoint(event)));
      else if (from && pressed && !moved) o.onConnectTap?.(from);
      reset(); o.setConnectFrom(null); o.setMode('idle');
    },
  };
}
