import { useEffect, useRef, useState } from 'react';
import type { Dispatch, PointerEvent as ReactPointerEvent, RefObject, SetStateAction } from 'react';
import type { Note } from '../../../shared/engine_contract';
import { CARD_HEIGHT, CARD_WIDTH, cardRect, clampCanvasDelta, type CanvasPoint } from '../model/align';
import { bounds } from '../model/motion';
import { SNAP_PIXELS, snapRect, type SnapGuide } from '../model/snap';
import type { Camera } from '../model/motion';
import type { CanvasMode } from './useCanvasSelection';

type Positions = Record<string, CanvasPoint>;
type Drag = { start: CanvasPoint; origin: CanvasPoint; positions: Positions; shift: boolean };
type Marquee = { a: CanvasPoint; b: CanvasPoint } | null;
const COAST_WINDOW_MS = 120;
/** 이 거리(화면 px) 넘게 움직여야 카드 끌기로 본다. 그 아래는 클릭. */
const DRAG_SLOP_PX = 3;

const noteAt = (event: { clientX: number; clientY: number }) => document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-note-id]')?.dataset.noteId;

/**
 * 캔버스 포인터 상호작용 상태 기계: 화면 끌기(관성), 카드 끌기(프로젝트에 놓기), 범위 선택, 연결 잇기.
 * 카드 끌기는 다른 카드 선에 붙고(Ctrl을 누르면 끔) 붙은 선을 guides로 내보낸다.
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
  onCardsMoved?: (ids: string[]) => void;
  onMovePositions: (next: Positions) => void; onMoveInto: (ids: string[], parent: string, from: Positions) => void; onConnect: (from: string, to: string) => void;
  /** 잇기를 빈 곳에 놓았다: 그 자리(캔버스 좌표)에 새 가지를 만든다. */
  onConnectEmpty?: (from: string, at: CanvasPoint) => void;
  /** 손잡이를 끌지 않고 눌렀다 뗐다(포인터를 보드가 잡아 버튼 click이 오지 않으므로 여기서 받는다). */
  onConnectTap?: (from: string) => void;
}) {
  const o = options;
  const [drag, setDrag] = useState<Drag | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const panSamples = useRef<Array<{ x: number; y: number; at: number }>>([]);
  const cardPointer = useRef<CanvasPoint | null>(null);
  const cardDown = useRef<{ point: CanvasPoint; moved: boolean } | null>(null);
  const snapOff = useRef(false);
  const [guides, setGuides] = useState<SnapGuide[]>([]);
  /** 잇기 미리보기: 포인터(캔버스 좌표)와 그 아래 카드. moved는 누른 채 문턱을 넘었는지(아니면 손잡이 클릭). */
  const [connectPreview, setConnectPreview] = useState<{ to: CanvasPoint; target?: string } | null>(null);
  const connectDown = useRef<{ point: CanvasPoint; moved: boolean } | null>(null);

  // 보드 원점은 누를 때만 잰다. 끌기 중 매 이동마다 getBoundingClientRect를 부르면 강제 레이아웃으로 프레임당 수 ms가 든다.
  const boardOrigin = useRef<{ left: number; top: number } | null>(null);
  const measureBoard = () => { const rect = o.host.current?.getBoundingClientRect(); boardOrigin.current = { left: rect?.left ?? 0, top: rect?.top ?? 0 }; return boardOrigin.current; };
  const screenPoint = (event: { clientX: number; clientY: number }, fresh = false): CanvasPoint => {
    const origin = fresh || !boardOrigin.current ? measureBoard() : boardOrigin.current;
    return { x: event.clientX - origin.left, y: event.clientY - origin.top };
  };
  const worldPoint = (screen: CanvasPoint): CanvasPoint => ({ x: screen.x / o.camera.zoom + o.camera.x, y: screen.y / o.camera.zoom + o.camera.y });
  const snapped = (current: Drag, dx: number, dy: number): { delta: CanvasPoint; guides: SnapGuide[] } => {
    const origins = Object.values(current.positions);
    const raw = clampCanvasDelta(origins, dx, dy);
    if (snapOff.current || !origins.length) return { delta: raw, guides: [] };
    const others = o.notes.filter((note) => !current.positions[note.id] && o.positions[note.id]).map((note) => cardRect(o.positions[note.id]));
    const moving = bounds(origins.map((point) => cardRect({ x: point.x + raw.x, y: point.y + raw.y })));
    if (!moving) return { delta: raw, guides: [] };
    const fit = snapRect(moving, others, SNAP_PIXELS / o.camera.zoom);
    return { delta: clampCanvasDelta(origins, raw.x + fit.dx, raw.y + fit.dy), guides: fit.guides };
  };
  const draggedTo = (current: Drag, pointer: CanvasPoint): Positions => {
    const world = worldPoint(pointer);
    const { delta, guides: found } = snapped(current, world.x - current.start.x, world.y - current.start.y);
    setGuides(found);
    return Object.fromEntries(Object.entries(current.positions).map(([id, original]) => [id, { x: original.x + delta.x, y: original.y + delta.y }]));
  };
  const reset = () => { cardDown.current = null; connectDown.current = null; setConnectPreview(null); setGuides([]); o.setAutopan(null); o.setConnectFrom(null); setDropTarget(null); o.setMode('idle'); setDrag(null); o.setMarquee(null); };
  const beginPan = (event: ReactPointerEvent) => {
    o.setMode('pan'); panSamples.current = [{ x: event.clientX, y: event.clientY, at: performance.now() }];
    setDrag({ start: { x: event.clientX, y: event.clientY }, origin: { x: o.camera.x, y: o.camera.y }, positions: {}, shift: false });
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  };
  const wantsPan = (event: ReactPointerEvent) => event.button === 1 || o.isPanHeld();

  // 자동 이동으로 카메라가 움직이는 동안 끌고 있는 카드가 포인터를 따라가게 한다.
  // biome-ignore lint/correctness/useExhaustiveDependencies: 카메라 이동이 다시 계산할 계기다. 나머지는 매 계산 때 최신 값을 읽는다.
  useEffect(() => {
    if (o.mode !== 'cards' || !drag || !cardPointer.current || !cardDown.current?.moved) return;
    const moved = draggedTo(drag, cardPointer.current);
    o.setDraftPositions((current) => ({ ...current, ...moved }));
  }, [o.camera, drag, o.mode]);

  return {
    dropTarget,
    guides,
    /** Esc: 진행 중 상호작용을 취소했으면 true. */
    cancel: () => {
      if (o.mode === 'idle') return false;
      if (o.mode === 'cards' && drag) o.setDraftPositions((current) => ({ ...current, ...drag.positions }));
      reset();
      return true;
    },
    beginConnection: (id: string, event?: ReactPointerEvent) => {
      o.setConnectFrom(id); o.setMode('connect');
      if (!event) return;
      connectDown.current = { point: screenPoint(event, true), moved: false };
      o.host.current?.setPointerCapture(event.pointerId);
    },
    connectPreview: o.mode === 'connect' && connectDown.current?.moved ? connectPreview : null,
    board: {
      onPointerDown: (event: ReactPointerEvent) => {
        if (wantsPan(event)) { beginPan(event); return; }
        if (event.target !== event.currentTarget) return;
        if (!event.shiftKey) o.setSelected([]);
        const screen = screenPoint(event, true); o.setMode('marquee'); o.setMarquee({ a: screen, b: screen });
        setDrag({ start: screen, origin: screen, positions: {}, shift: event.shiftKey }); event.currentTarget.setPointerCapture(event.pointerId);
      },
      onPointerMove: (event: ReactPointerEvent) => {
        if (o.mode === 'connect') {
          const down = connectDown.current;
          if (!down || !o.connectFrom) return;
          const point = screenPoint(event);
          if (!down.moved && Math.hypot(point.x - down.point.x, point.y - down.point.y) <= DRAG_SLOP_PX) return;
          down.moved = true;
          const world = worldPoint(point);
          // 놓을 카드는 위치로 판정한다(이동마다 elementFromPoint를 부르지 않는다). 위에 그려진 카드부터.
          const target = [...o.notes].reverse().find((note) => note.id !== o.connectFrom && o.positions[note.id]
            && world.x >= o.positions[note.id].x && world.x <= o.positions[note.id].x + CARD_WIDTH && world.y >= o.positions[note.id].y && world.y <= o.positions[note.id].y + CARD_HEIGHT);
          setConnectPreview({ to: world, target: target?.id });
          return;
        }
        if (!drag) return;
        if (o.mode === 'pan') {
          const dx = event.clientX - drag.start.x; const dy = event.clientY - drag.start.y;
          o.setCamera((current) => ({ ...current, x: drag.origin.x - dx / current.zoom, y: drag.origin.y - dy / current.zoom }));
          const now = performance.now(); panSamples.current = [...panSamples.current.filter((sample) => now - sample.at <= COAST_WINDOW_MS), { x: event.clientX, y: event.clientY, at: now }];
        } else if (o.mode === 'cards') {
          snapOff.current = event.ctrlKey;
          const point = screenPoint(event); cardPointer.current = point; o.setAutopan([point.x, point.y]);
          const down = cardDown.current;
          if (down && !down.moved && Math.hypot(point.x - down.point.x, point.y - down.point.y) > DRAG_SLOP_PX) { down.moved = true; o.onCardsMoved?.(Object.keys(drag.positions)); }
          // 끌기 문턱을 넘기 전(단순 클릭)에는 카드를 옮기거나 붙이지 않아 안내선도 뜨지 않는다.
          if (down && !down.moved) return;
          // 놓을 프로젝트는 위치로 판정한다(elementFromPoint는 끌기 중 매 이동마다 강제 레이아웃을 일으킨다). 위에 그려진 카드부터 본다.
          const world = worldPoint(point);
          const project = [...o.notes].reverse().find((note) => note.canvas.project && !drag.positions[note.id] && o.positions[note.id]
            && world.x >= o.positions[note.id].x && world.x <= o.positions[note.id].x + CARD_WIDTH && world.y >= o.positions[note.id].y && world.y <= o.positions[note.id].y + CARD_HEIGHT);
          setDropTarget(project && o.canMoveInto(project.id, Object.keys(drag.positions)) ? project.id : null);
          o.setDraftPositions((current) => ({ ...current, ...draggedTo(drag, point) }));
        } else if (o.mode === 'marquee') o.setMarquee((current) => current ? { ...current, b: screenPoint(event) } : current);
      },
      onPointerUp: (event: ReactPointerEvent) => {
        o.setAutopan(null);
        if (o.mode === 'connect') {
          const target = noteAt(event);
          const down = connectDown.current;
          if (target && o.connectFrom && o.connectFrom !== target) o.onConnect(o.connectFrom, target);
          else if (!target && o.connectFrom && down?.moved) o.onConnectEmpty?.(o.connectFrom, worldPoint(screenPoint(event)));
          else if (o.connectFrom && down && !down.moved) o.onConnectTap?.(o.connectFrom);
          connectDown.current = null; setConnectPreview(null);
          o.setConnectFrom(null); o.setMode('idle');
          return;
        }
        if (o.mode === 'pan' && panSamples.current.length > 1) {
          const [first] = panSamples.current; const last = panSamples.current[panSamples.current.length - 1]; const elapsed = (last.at - first.at) / 1000;
          if (elapsed > 0) o.startCoast([(last.x - first.x) / elapsed, (last.y - first.y) / elapsed]);
        }
        panSamples.current = []; cardPointer.current = null; cardDown.current = null;
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
        o.setMode('idle'); setDrag(null); o.setMarquee(null); setDropTarget(null); setGuides([]);
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
      cardPointer.current = screenPoint(event, true); cardDown.current = { point: cardPointer.current, moved: false };
      setDrag({ start: worldPoint(screenPoint(event)), origin: { x: 0, y: 0 }, positions: Object.fromEntries(next.map((item) => [item, o.positions[item]])), shift: event.shiftKey });
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    },
  };
}
