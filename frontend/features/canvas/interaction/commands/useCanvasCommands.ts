import { useMemo } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import type { Note } from '../../../shared/engine_contract';
import type { MemoStore } from '../../../shared/useMemoivy';
import { alignPositions, CARD_HEIGHT, CARD_WIDTH, canvasSlot, cardRect, changedPositions, clampCanvasDelta, savedPoint, type Alignment, type CanvasPoint, vacantGroup } from '../model/align';
import { bounds } from '../model/motion';
import { branchRoot, buildGraph, scopeEdges, treeParent } from '../model/links/graph';
import { DEFAULT_COLUMN_GAP as BRANCH_GAP_X, DEFAULT_ROW_GAP as BRANCH_GAP_Y, mindmapLayout } from '../model/links/mindmap_layout';
import { useCanvasHistory, type CanvasChange } from './useCanvasHistory';

type Positions = Record<string, CanvasPoint>;
const NUDGE: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };

/** 캔버스 변경 명령: 저장·실행취소 기록과 정렬·이동·묶기·완료·연결. 화면 상태(포인터·카메라)는 모른다. */
export function useCanvasCommands({ store, notes, positions, selected, activeScope, setDraftPositions, triggerLink }: {
  store: MemoStore; notes: Note[]; positions: Positions; selected: string[]; activeScope: string | null;
  setDraftPositions: Dispatch<SetStateAction<Positions>>; triggerLink: (key: string, fresh: boolean) => void;
}) {
  const persist = (changes: CanvasChange[]) => {
    const moved = Object.fromEntries(changes.flatMap(({ id, canvas }) => canvas.position ? [[id, { x: canvas.position[0], y: canvas.position[1] }]] : []));
    if (Object.keys(moved).length) setDraftPositions((current) => ({ ...current, ...moved }));
    void store.mutateCanvas(changes).catch(() => undefined);
  };
  const states = useMemo(() => Object.fromEntries(notes.map((note) => [note.id, { ...note.canvas, position: note.canvas.position ?? [positions[note.id].x, positions[note.id].y] }])), [notes, positions]);
  const history = useCanvasHistory(states, persist);
  const commit = (changes: CanvasChange[]) => { if (!changes.length) return; history.record(changes); persist(changes); };

  const commitPositions = (next: Positions) => commit(changedPositions(next, Object.fromEntries(notes.map((note) => [note.id, note.canvas.position])))
    .map(({ id, position }) => ({ id, canvas: { position } })));
  /** 다른 범위로 옮길 때 그곳의 빈자리에 묶음 그대로 놓는다. */
  const moveInto = (ids: string[], parent: string | null, from: Positions = positions) => {
    const occupied = store.notes.filter((note) => (note.canvas.parent ?? null) === parent && !ids.includes(note.id))
      .map((note, index) => note.canvas.position ? { x: note.canvas.position[0], y: note.canvas.position[1] } : canvasSlot(index));
    const destination = vacantGroup(ids, from, occupied);
    commit(ids.flatMap((id) => destination[id] ? [{ id, canvas: { parent, position: savedPoint(destination[id]) } }] : []));
  };
  const link = (source: Note, target: string) => {
    const next = source.canvas.next.includes(target) ? source.canvas.next.filter((id) => id !== target) : [...source.canvas.next, target];
    if (next.length > source.canvas.next.length) triggerLink(`${source.id}:${target}`, true);
    commit([{ id: source.id, canvas: { next } }]);
  };

  /** 선택 카드의 가지 뿌리부터 next 트리를 오른쪽으로 뻗게 놓는다. 여러 부모면 첫 부모 기준. */
  const arrangeMindmap = (): Positions => {
    const seed = selected[0];
    if (!seed || !positions[seed]) return {};
    const graph = buildGraph(scopeEdges(store.notes, activeScope));
    const root = branchRoot(graph, seed);
    if (!positions[root]) return {};
    return mindmapLayout(root, (id) => (graph.out.get(id) ?? []).filter((child) => positions[child]), {}, positions[root]);
  };

  /**
   * Tab·Enter 가지 만들기: kind=child면 선택 카드 아래 가지, sibling이면 같은 부모의 형제.
   * 새 메모는 부모 오른쪽 열의 마지막 가지 아래에 놓는다. 만들 수 없으면(형제인데 부모 없음) undefined.
   */
  const addBranch = async (kind: 'child' | 'sibling', from?: { seed: string; at?: CanvasPoint }): Promise<Note | undefined> => {
    const seed = from?.seed ?? (selected.length === 1 ? selected[0] : undefined);
    if (!seed || !positions[seed]) return undefined;
    const graph = buildGraph(scopeEdges(store.notes, activeScope));
    const parentId = kind === 'child' ? seed : treeParent(graph, seed);
    const parent = parentId ? notes.find((note) => note.id === parentId) : undefined;
    if (!parent || !positions[parent.id]) return undefined;
    const siblings = parent.canvas.next.flatMap((id) => positions[id] ? [positions[id]] : []);
    const base = positions[parent.id];
    // 끌어서 빈 곳에 놓았으면 그 자리(포인터가 카드 왼쪽 가운데), 아니면 마지막 가지 아래.
    const position = from?.at ? { x: from.at.x, y: from.at.y - CARD_HEIGHT / 2 } : siblings.length
      ? { x: Math.max(...siblings.map((point) => point.x)), y: Math.max(...siblings.map((point) => point.y)) + CARD_HEIGHT + BRANCH_GAP_Y }
      : { x: base.x + CARD_WIDTH + BRANCH_GAP_X, y: base.y };
    const created = await store.createNote({ parent: activeScope });
    if (!created) return undefined;
    triggerLink(`${parent.id}:${created.id}`, true);
    commit([{ id: parent.id, canvas: { next: [...parent.canvas.next, created.id] } }, { id: created.id, canvas: { position: savedPoint(position) } }]);
    return created;
  };

  return {
    addBranch,
    /** 형제를 만들 부모가 있는지(Enter를 가로챌지) 판정한다. */
    canAddSibling: () => selected.length === 1 && Boolean(treeParent(buildGraph(scopeEdges(store.notes, activeScope)), selected[0])),
    history,
    commit,
    commitPositions,
    moveInto,
    link,
    align: (kind: Alignment) => {
      if (kind === 'mindmap') { commitPositions(arrangeMindmap()); return; }
      const ids = kind === 'flow' && !selected.length ? notes.map((note) => note.id) : selected;
      const edges = notes.flatMap((note) => note.canvas.next.map((target): [string, string] => [note.id, target]));
      commitPositions(alignPositions(ids, positions, kind, edges));
    },
    nudge: (key: string, step: number) => {
      const [dx, dy] = NUDGE[key] ?? [0, 0];
      const group = selected.flatMap((id) => positions[id] ? [positions[id]] : []);
      const delta = clampCanvasDelta(group, dx * step, dy * step);
      commitPositions(Object.fromEntries(selected.flatMap((id) => positions[id] ? [[id, { x: positions[id].x + delta.x, y: positions[id].y + delta.y }]] : [])));
    },
    toggleDone: (ids: string[]) => {
      const targets = notes.filter((note) => ids.includes(note.id));
      const done = targets.some((note) => !note.canvas.done);
      commit(targets.map((note) => ({ id: note.id, canvas: { done } })));
    },
    /** 선택을 새 프로젝트로 묶는다. 만든 프로젝트를 돌려준다. */
    group: async (): Promise<Note | undefined> => {
      const area = bounds(selected.flatMap((id) => positions[id] ? [cardRect(positions[id])] : []));
      if (!area) return undefined;
      const project = await store.createNote({ project: true, parent: activeScope });
      if (!project) return undefined;
      commit([{ id: project.id, canvas: { position: savedPoint(area) } }, ...selected.map((id) => ({ id, canvas: { parent: project.id } }))]);
      return project;
    },
  };
}
