import { useState } from 'react';
import type { CanvasNode, Note } from '../../../../shared/engine_contract';

export type CanvasChange = { id: string; canvas: Partial<CanvasNode> };
/**
 * 명령 한 면이 저장소에 하는 일. 캔버스 필드 변경에 더해 삭제(`remove`)와 그 역인 복원(`restore`)을 같은 틀로 담아
 * 되돌리기 규칙 하나로 처리한다. 복원은 `restore` 메모를 원래 id로 되살리고 `changes`로 남은 메모의 연결·부모를 되돌린다.
 */
export type Step = { changes: CanvasChange[]; restore?: Note[]; remove?: string[] };
export type Command = { before: Step; after: Step };
type CanvasState = Pick<CanvasNode, 'position' | 'next' | 'done' | 'parent'>;
/** 실행 취소 기록 상한. 오래 써도 기록이 끝없이 쌓이지 않게 가장 오래된 것부터 버린다. */
export const HISTORY_LIMIT = 200;
export const pushBounded = <T,>(stack: readonly T[], item: T, limit = HISTORY_LIMIT): T[] => [...stack, item].slice(-limit);

/**
 * 기록한 변경을 지금 있는 메모에만 맞춘다. 엔진 스냅샷이 단일 출처라, 다른 경로(삭제·다른 창)로 사라진 메모와
 * 그 메모로 가는 연결은 되돌리기에서 빠진다. 시간 O(변경 수 + 연결 수), 공간 O(변경 수).
 */
export function liveChanges(changes: readonly CanvasChange[], exists: ReadonlySet<string>): CanvasChange[] {
  return changes.flatMap(({ id, canvas }) => {
    if (!exists.has(id)) return [];
    const next = canvas.next?.filter((target) => exists.has(target));
    return [{ id, canvas: next ? { ...canvas, next } : canvas }];
  });
}

/** 지금 저장소에 적용할 수 있는 부분만 남긴다. 적용할 것이 없으면 undefined. */
export function liveStep(step: Step, exists: ReadonlySet<string>): Step | undefined {
  if (step.restore) {
    // 이미 다시 생긴 메모가 있으면 복원은 겹친다. 되살릴 메모는 연결 대상으로 본다.
    if (step.restore.some((note) => exists.has(note.id))) return undefined;
    const present = new Set([...exists, ...step.restore.map((note) => note.id)]);
    return { restore: step.restore, changes: liveChanges(step.changes, present) };
  }
  if (step.remove) {
    const remove = step.remove.filter((id) => exists.has(id));
    return remove.length ? { changes: [], remove } : undefined;
  }
  const changes = liveChanges(step.changes, exists);
  return changes.length ? { changes } : undefined;
}

/** 스택 끝에서부터 아직 적용할 수 있는 명령을 찾는다. 적용할 것이 없어진 명령은 버린다. */
export function popLive(stack: readonly Command[], side: keyof Command, exists: ReadonlySet<string>) {
  for (let index = stack.length - 1; index >= 0; index--) {
    const step = liveStep(stack[index][side], exists);
    if (step) return { rest: stack.slice(0, index), command: stack[index], step };
  }
  return { rest: [] as Command[], command: undefined, step: undefined };
}

/**
 * 삭제 명령. 엔진 삭제는 자식을 위 부모로 올리고 남은 메모의 연결을 지우므로, 그 두 필드를 삭제 전 값으로 담는다.
 * 시간 O(메모 수 + 연결 수), 공간 O(영향받은 메모 수).
 */
export function deletionCommand(notes: readonly Note[], ids: readonly string[]): Command {
  const removed = new Set(ids);
  const restore = notes.filter((note) => removed.has(note.id));
  const changes = notes.flatMap((note) => {
    if (removed.has(note.id)) return [];
    const parent = note.canvas.parent ?? null;
    const touched = (parent !== null && removed.has(parent)) || note.canvas.next.some((id) => removed.has(id));
    return touched ? [{ id: note.id, canvas: { parent, next: note.canvas.next } }] : [];
  });
  return { before: { restore, changes }, after: { changes: [], remove: restore.map((note) => note.id) } };
}

/** `current`는 지금 보이는 범위의 상태, `exists`는 저장소 전체 메모 id(범위 밖 기록을 지우지 않도록 분리한다). */
export function useCanvasHistory(current: Record<string, CanvasState>, persist: (step: Step) => void, exists: ReadonlySet<string>) {
  const [undoStack, setUndoStack] = useState<Command[]>([]);
  const [redoStack, setRedoStack] = useState<Command[]>([]);
  const record = (after: CanvasChange[]) => {
    const before = after.flatMap(({ id, canvas }) => {
      const state = current[id];
      if (!state) return [];
      const previous: Partial<CanvasNode> = {};
      for (const key of Object.keys(canvas) as Array<keyof CanvasState>) {
        if (key in state) Object.assign(previous, { [key]: state[key] });
      }
      return [{ id, canvas: previous }];
    });
    const actual = after.filter(({ id, canvas }) => Object.entries(canvas).some(([key, value]) => JSON.stringify(current[id]?.[key as keyof CanvasState]) !== JSON.stringify(value)));
    const actualBefore = before.filter((change) => actual.some((item) => item.id === change.id));
    if (!actual.length) return;
    push({ before: { changes: actualBefore }, after: { changes: actual } });
  };
  const push = (command: Command) => {
    setUndoStack((stack) => pushBounded(stack, command));
    setRedoStack([]);
  };
  // 되돌리기·다시 하기는 같은 규칙의 거울상이다: 한쪽 스택에서 살아 있는 명령을 꺼내 반대쪽에 쌓고 그 면을 적용한다.
  const replay = (from: Command[], side: keyof Command, setFrom: typeof setUndoStack, setTo: typeof setUndoStack) => {
    const { rest, command, step } = popLive(from, side, exists);
    setFrom(rest);
    if (!command || !step) return;
    setTo((stack) => pushBounded(stack, command));
    persist(step);
  };
  const undo = () => replay(undoStack, 'before', setUndoStack, setRedoStack);
  const redo = () => replay(redoStack, 'after', setRedoStack, setUndoStack);
  return { record, push, undo, redo, canUndo: undoStack.length > 0, canRedo: redoStack.length > 0 };
}
