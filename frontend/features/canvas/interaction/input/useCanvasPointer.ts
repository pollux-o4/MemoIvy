import { useEffect, useRef, useState } from 'react';
import type { Dispatch, PointerEvent as ReactPointerEvent, RefObject, SetStateAction } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import type { CanvasPoint } from '../../model/align';
import { dragDelta, keepRecent, type PointerSample, releaseVelocity } from '../../model/drag';
import { cardsInRange, topCardAt } from '../../model/hit';
import { useBoardPoint } from '../camera/useBoardPoint';
import { type ConnectOutcome, DRAG_SLOP_PX, useConnectGesture } from './useConnectGesture';
import type { SnapGuide } from '../../model/snap';
import type { Camera } from '../../model/motion';
import type { CanvasMode } from '../state/useCanvasSelection';

type Positions = Record<string, CanvasPoint>;
type Drag = { start: CanvasPoint; origin: CanvasPoint; positions: Positions; shift: boolean };
type Marquee = { a: CanvasPoint; b: CanvasPoint } | null;

const noteAt = (event: { clientX: number; clientY: number }) => document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('[data-note-id]')?.dataset.noteId;

/**
 * 캔버스 포인터 상호작용 상태 기계: 화면 끌기(관성), 카드 끌기(프로젝트에 놓기), 범위 선택, 연결 잇기.
 * 카드 끌기는 다른 카드 선에 붙고(Ctrl을 누르면 끔) 붙은 선을 guides로 내보낸다.
 * 결과 반영은 주입된 명령(onMovePositions·onMoveInto·onConnect)으로만 한다.
 */
// shortcut: 잇기만 useConnectGesture로 분리, 화면 끌기·카드 끌기·범위 선택은 아직 mode 분기, 제스처가 하나 더 늘면 모드별 제스처 객체(State)로 나눈다.
export function useCanvasPointer(options: {
  host: RefObject<HTMLDivElement | null>; camera: Camera; setCamera: (update: (current: Camera) => Camera) => void;
  setAutopan: (screen: [number, number] | null) => void; startCoast: (velocity: [number, number]) => void; isPanHeld: () => boolean;
  mode: CanvasMode; setMode: (mode: CanvasMode) => void; marquee: Marquee; setMarquee: Dispatch<SetStateAction<Marquee>>;
  connectFrom: string | null; setConnectFrom: (id: string | null) => void;
  selected: string[]; setSelected: Dispatch<SetStateAction<string[]>>;
  notes: Note[]; allNotes: Note[]; positions: Positions; draftPositions: Positions; setDraftPositions: Dispatch<SetStateAction<Positions>>;
  canMoveInto: (target: string, ids: string[]) => boolean;
  onCardsMoved?: (ids: string[]) => void;
  onMovePositions: (next: Positions) => void; onMoveInto: (ids: string[], parent: string, from: Positions) => void;
} & ConnectOutcome) {
  const o = options;
  const [drag, setDrag] = useState<Drag | null>(null);
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const panSamples = useRef<PointerSample[]>([]);
  const cardPointer = useRef<CanvasPoint | null>(null);
  const cardDown = useRef<{ point: CanvasPoint; moved: boolean } | null>(null);
  const snapOff = useRef(false);
  const [guides, setGuides] = useState<SnapGuide[]>([]);
  const point = useBoardPoint(o.host, o.camera);
  const { screenPoint, worldPoint } = point;
  const connect = useConnectGesture({ ...o, point });
  const snapped = (current: Drag, dx: number, dy: number) => {
    const others = o.notes.flatMap((note) => !current.positions[note.id] && o.positions[note.id] ? [o.positions[note.id]] : []);
    return dragDelta(Object.values(current.positions), others, dx, dy, o.camera.zoom, snapOff.current);
  };
  const draggedTo = (current: Drag, pointer: CanvasPoint): Positions => {
    const world = worldPoint(pointer);
    const { delta, guides: found } = snapped(current, world.x - current.start.x, world.y - current.start.y);
    setGuides(found);
    return Object.fromEntries(Object.entries(current.positions).map(([id, original]) => [id, { x: original.x + delta.x, y: original.y + delta.y }]));
  };
  const reset = () => { cardDown.current = null; connect.reset(); setGuides([]); o.setAutopan(null); o.setConnectFrom(null); setDropTarget(null); o.setMode('idle'); setDrag(null); o.setMarquee(null); };
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
    beginConnection: connect.begin,
    connectPreview: o.mode === 'connect' ? connect.preview : null,
    connectPicking: o.mode === 'connect' && connect.picking,
    board: {
      onPointerDownCapture: (event: ReactPointerEvent) => {
        if (o.mode !== 'connect' || !connect.picking || event.button !== 0) return;
        event.preventDefault(); event.stopPropagation();
        connect.pick(noteAt(event), event);
      },
      onPointerDown: (event: ReactPointerEvent) => {
        if (wantsPan(event)) { beginPan(event); return; }
        if (event.target !== event.currentTarget) return;
        if (!event.shiftKey) o.setSelected([]);
        const screen = screenPoint(event, true); o.setMode('marquee'); o.setMarquee({ a: screen, b: screen });
        setDrag({ start: screen, origin: screen, positions: {}, shift: event.shiftKey }); event.currentTarget.setPointerCapture(event.pointerId);
      },
      onPointerMove: (event: ReactPointerEvent) => {
        if (o.mode === 'connect') { connect.move(event); return; }
        if (!drag) return;
        if (o.mode === 'pan') {
          const dx = event.clientX - drag.start.x; const dy = event.clientY - drag.start.y;
          o.setCamera((current) => ({ ...current, x: drag.origin.x - dx / current.zoom, y: drag.origin.y - dy / current.zoom }));
          panSamples.current = keepRecent(panSamples.current, { x: event.clientX, y: event.clientY, at: performance.now() });
        } else if (o.mode === 'cards') {
          snapOff.current = event.ctrlKey;
          const point = screenPoint(event); cardPointer.current = point; o.setAutopan([point.x, point.y]);
          const down = cardDown.current;
          if (down && !down.moved && Math.hypot(point.x - down.point.x, point.y - down.point.y) > DRAG_SLOP_PX) { down.moved = true; o.onCardsMoved?.(Object.keys(drag.positions)); }
          // 끌기 문턱을 넘기 전(단순 클릭)에는 카드를 옮기거나 붙이지 않아 안내선도 뜨지 않는다.
          if (down && !down.moved) return;
          const world = worldPoint(point);
          const project = topCardAt(world, o.notes, o.positions, (note) => Boolean(note.canvas.project) && !drag.positions[note.id]);
          setDropTarget(project && o.canMoveInto(project.id, Object.keys(drag.positions)) ? project.id : null);
          o.setDraftPositions((current) => ({ ...current, ...draggedTo(drag, point) }));
        } else if (o.mode === 'marquee') o.setMarquee((current) => current ? { ...current, b: screenPoint(event) } : current);
      },
      onPointerUp: (event: ReactPointerEvent) => {
        o.setAutopan(null);
        if (o.mode === 'connect') { connect.end(noteAt(event), event); return; }
        const velocity = o.mode === 'pan' ? releaseVelocity(panSamples.current) : null;
        if (velocity) o.startCoast(velocity);
        panSamples.current = []; cardPointer.current = null; cardDown.current = null;
        if (o.mode === 'marquee' && o.marquee) {
          const inside = cardsInRange(worldPoint(o.marquee.a), worldPoint(o.marquee.b), o.notes, o.positions);
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
