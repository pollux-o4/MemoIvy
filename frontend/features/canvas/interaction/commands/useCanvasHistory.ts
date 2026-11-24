import { useState } from 'react';
import type { CanvasNode } from '../../../../shared/engine_contract';

export type CanvasChange = { id: string; canvas: Partial<CanvasNode> };
export type Command = { before: CanvasChange[]; after: CanvasChange[] };
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

/** 스택 끝에서부터 아직 적용할 수 있는 명령을 찾는다. 적용할 것이 없어진 명령은 버린다. */
export function popLive(stack: readonly Command[], side: keyof Command, exists: ReadonlySet<string>) {
  for (let index = stack.length - 1; index >= 0; index--) {
    const changes = liveChanges(stack[index][side], exists);
    if (changes.length) return { rest: stack.slice(0, index), command: stack[index], changes };
  }
  return { rest: [] as Command[], command: undefined, changes: [] as CanvasChange[] };
}

/** `current`는 지금 보이는 범위의 상태, `exists`는 저장소 전체 메모 id(범위 밖 기록을 지우지 않도록 분리한다). */
export function useCanvasHistory(current: Record<string, CanvasState>, persist: (changes: CanvasChange[]) => void, exists: ReadonlySet<string>) {
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
    setUndoStack((stack) => pushBounded(stack, { before: actualBefore, after: actual }));
    setRedoStack([]);
  };
  // 되돌리기·다시 하기는 같은 규칙의 거울상이다: 한쪽 스택에서 살아 있는 명령을 꺼내 반대쪽에 쌓고 그 면을 적용한다.
  const replay = (from: Command[], side: keyof Command, setFrom: typeof setUndoStack, setTo: typeof setUndoStack) => {
    const { rest, command, changes } = popLive(from, side, exists);
    setFrom(rest);
    if (!command) return;
    setTo((stack) => pushBounded(stack, command));
    persist(changes);
  };
  const undo = () => replay(undoStack, 'before', setUndoStack, setRedoStack);
  const redo = () => replay(redoStack, 'after', setRedoStack, setUndoStack);
  return { record, undo, redo, canUndo: undoStack.length > 0, canRedo: redoStack.length > 0 };
}
