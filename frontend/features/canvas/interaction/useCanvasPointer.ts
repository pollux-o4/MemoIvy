import { useEffect, useRef, useState } from 'react';
import type { Dispatch, PointerEvent as ReactPointerEvent, RefObject, SetStateAction } from 'react';
import type { Note } from '../../../shared/engine_contract';
import { CARD_HEIGHT, CARD_WIDTH, clampCanvasDelta, type CanvasPoint } from '../model/align';
import type { Camera } from '../model/motion';
import type { CanvasMode } from './useCanvasSelection';

type Positions = Record<string, CanvasPoint>;
type Drag = { start: CanvasPoint; origin: CanvasPoint; positions: Positions; shift: boolean };
type Marquee = { a: CanvasPoint; b: CanvasPoint } | null;
const COAST_WINDOW_MS = 120;

const noteAt = (event: { clientX: number; clientY: number }) => document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-note-id]')?.dataset.noteId;

/**
 * 캔버스 포인터 상호작용 상태 기계: 화면 끌기(관성), 카드 끌기(프로젝트에 놓기), 범위 선택, 연결 잇기.
 * 결과 반영은 주입된 명령(onMovePositions·onMoveInto·onConnect)으로만 한다.
 */
export function useCanvasPointer(options: {
  host: RefObject<HTMLDivElement | null>; camera: Camera; setCamera: (update: (current: Camera) => Camera) => void;
  setAutopan: (screen: [number, number] | null) => void; startCoast: (velocity: [number, number]) => void; isPanHeld: () => boolean;
  mode: CanvasMode; setMode: (mode: CanvasMode) => void; marquee: Marquee; setMarquee: Dispatch<SetStateAction<Marquee>>;
  connectFrom: string | null; setConnectFrom: (id: string | null) => void;
  selected: string[]; setSelected: Dispatch<SetStateAction<string[]>>;
  notes: Note[]; allNotes: Note[]; positions: Positions; draftPositions: Positions; setDraftPositions: Dispatch<SetStateAction<Positions>>;
  canMoveInto: (target: string, ids: string[]) => boolean;
  onMovePositions: (next: Positions) => void; onMoveInto: (ids: string[], parent: string, from: Positions) => void; onConnect: (from: string, to: string) => void;
}) {
  const o = options;
  const [drag, setDrag] = useState<Drag | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const panSamples = useRef<Array<{ x: number; y: number; at: number }>>([]);
  const cardPointer = useRef<CanvasPoint | null>(null);

  const screenPoint = (event: { clientX: number; clientY: number }): CanvasPoint => { const rect = o.host.current?.getBoundingClientRect(); return { x: event.clientX - (rect?.left ?? 0), y: event.clientY - (rect?.top ?? 0) }; };
  const worldPoint = (screen: CanvasPoint): CanvasPoint => ({ x: screen.x / o.camera.zoom + o.camera.x, y: screen.y / o.camera.zoom + o.camera.y });
  const draggedTo = (current: Drag, pointer: CanvasPoint): Positions => {
    const world = worldPoint(pointer);
    const delta = clampCanvasDelta(Object.values(current.positions), world.x - current.start.x, world.y - current.start.y);
    return Object.fromEntries(Object.entries(current.positions).map(([id, original]) => [id, { x: original.x + delta.x, y: original.y + delta.y }]));
  };
  const reset = () => { o.setAutopan(null); o.setConnectFrom(null); setDropTarget(null); o.setMode('idle'); setDrag(null); o.setMarquee(null); };
  const beginPan = (event: ReactPointerEvent) => {
    o.setMode('pan'); panSamples.current = [{ x: event.clientX, y: event.clientY, at: performance.now() }];
    setDrag({ start: { x: event.clientX, y: event.clientY }, origin: { x: o.camera.x, y: o.camera.y }, positions: {}, shift: false });
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };
  const wantsPan = (event: ReactPointerEvent) => event.button === 1 || o.isPanHeld();

  // 자동 이동으로 카메라가 움직이는 동안 끌고 있는 카드가 포인터를 따라가게 한다.
  useEffect(() => {
    if (o.mode !== 'cards' || !drag || !cardPointer.current) return;
    const moved = draggedTo(drag, cardPointer.current);
    o.setDraftPositions((current) => ({ ...current, ...moved }));
  }, [o.camera, drag, o.mode]);

  return {
    dropTarget,
    /** Esc: 진행 중 상호작용을 취소했으면 true. */
    cancel: () => {
      if (o.mode === 'idle') return false;
      if (o.mode === 'cards' && drag) o.setDraftPositions((current) => ({ ...current, ...drag.positions }));
      reset();
      return true;
    },
    beginConnection: (id: string) => { o.setConnectFrom(id); o.setMode('connect'); },
    board: {
      onPointerDown: (event: ReactPointerEvent) => {
        if (wantsPan(event)) { beginPan(event); return; }
        if (event.target !== event.currentTarget) return;
        if (!event.shiftKey) o.setSelected([]);
        const screen = screenPoint(event); o.setMode('marquee'); o.setMarquee({ a: screen, b: screen });
        setDrag({ start: screen, origin: screen, positions: {}, shift: event.shiftKey }); event.currentTarget.setPointerCapture(event.pointerId);
      },
      onPointerMove: (event: ReactPointerEvent) => {
        if (o.mode === 'connect' || !drag) return;
        if (o.mode === 'pan') {
          const dx = event.clientX - drag.start.x; const dy = event.clientY - drag.start.y;
          o.setCamera((current) => ({ ...current, x: drag.origin.x - dx / current.zoom, y: drag.origin.y - dy / current.zoom }));
          const now = performance.now(); panSamples.current = [...panSamples.current.filter((sample) => now - sample.at <= COAST_WINDOW_MS), { x: event.clientX, y: event.clientY, at: now }];
        } else if (o.mode === 'cards') {
          const point = screenPoint(event); cardPointer.current = point; o.setAutopan([point.x, point.y]);
          const over = noteAt(event);
          const project = o.allNotes.find((note) => note.id === over && note.canvas.project);
          setDropTarget(project && o.canMoveInto(project.id, Object.keys(drag.positions)) ? project.id : null);
          o.setDraftPositions((current) => ({ ...current, ...draggedTo(drag, point) }));
        } else if (o.mode === 'marquee') o.setMarquee((current) => current ? { ...current, b: screenPoint(event) } : current);
      },
      onPointerUp: (event: ReactPointerEvent) => {
        o.setAutopan(null);
        if (o.mode === 'connect') {
          const target = noteAt(event);
          if (target && o.connectFrom && o.connectFrom !== target) o.onConnect(o.connectFrom, target);
          o.setConnectFrom(null); o.setMode('idle');
          return;
        }
        if (o.mode === 'pan' && panSamples.current.length > 1) {
          const [first] = panSamples.current; const last = panSamples.current.at(-1)!; const elapsed = (last.at - first.at) / 1000;
          if (elapsed > 0) o.startCoast([(last.x - first.x) / elapsed, (last.y - first.y) / elapsed]);
        }
        panSamples.current = []; cardPointer.current = null;
        if (o.mode === 'marquee' && o.marquee) {
          const a = worldPoint(o.marquee.a); const b = worldPoint(o.marquee.b);
          const left = Math.min(a.x, b.x); const top = Math.min(a.y, b.y); const right = Math.max(a.x, b.x); const bottom = Math.max(a.y, b.y);
          const inside = o.notes.filter((note) => { const point = o.positions[note.id]; return point.x < right && point.x + CARD_WIDTH > left && point.y < bottom && point.y + CARD_HEIGHT > top; }).map((note) => note.id);
          o.setSelected((current) => drag?.shift ? [...new Set([...current, ...inside])] : inside);
        }
        if (o.mode === 'cards' && drag) {
          const ids = Object.keys(drag.positions);
          const targetId = noteAt(event);
          const target = o.allNotes.find((note) => note.id === targetId && note.canvas.project);
          if (target && ids.length && o.canMoveInto(target.id, ids)) {
            o.onMoveInto(ids, target.id, Object.fromEntries(ids.flatMap((id) => (o.draftPositions[id] ?? o.positions[id]) ? [[id, o.draftPositions[id] ?? o.positions[id]]] : [])));
          } else {
            o.onMovePositions(Object.fromEntries(Object.entries(o.draftPositions).filter(([id, point]) => drag.positions[id] && (point.x !== drag.positions[id].x || point.y !== drag.positions[id].y))));
          }
        }
        o.setMode('idle'); setDrag(null); o.setMarquee(null); setDropTarget(null);
      },
      onPointerCancel: () => {
        if (o.mode === 'cards' && drag) o.setDraftPositions((current) => ({ ...current, ...drag.positions }));
        reset();
      },
    },
    beginCardDrag: (event: ReactPointerEvent, id: string) => {
      if (wantsPan(event)) { event.preventDefault(); event.stopPropagation(); beginPan(event); return; }
      if (event.button !== 0) return;
      event.stopPropagation();
      const next = event.shiftKey ? o.selected.includes(id) ? o.selected.filter((item) => item !== id) : [...o.selected, id] : o.selected.includes(id) ? o.selected : [id];
      o.setSelected(next); o.setMode('cards');
      cardPointer.current = screenPoint(event);
      setDrag({ start: worldPoint(screenPoint(event)), origin: { x: 0, y: 0 }, positions: Object.fromEntries(next.map((item) => [item, o.positions[item]])), shift: event.shiftKey });
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    },
  };
}
