import { useState } from 'react';
import type { CanvasNode } from '../../../shared/engine_contract';

export type CanvasChange = { id: string; canvas: Partial<CanvasNode> };
type Command = { before: CanvasChange[]; after: CanvasChange[] };
type CanvasState = Pick<CanvasNode, 'position' | 'next' | 'done' | 'parent'>;
/** 실행 취소 기록 상한. 오래 써도 기록이 끝없이 쌓이지 않게 가장 오래된 것부터 버린다. */
export const HISTORY_LIMIT = 200;
export const pushBounded = <T,>(stack: readonly T[], item: T, limit = HISTORY_LIMIT): T[] => [...stack, item].slice(-limit);

export function useCanvasHistory(current: Record<string, CanvasState>, persist: (changes: CanvasChange[]) => void) {
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
  const undo = () => {
    const command = undoStack.at(-1);
    if (!command) return;
    setUndoStack((stack) => stack.slice(0, -1));
    setRedoStack((stack) => pushBounded(stack, command));
    persist(command.before);
  };
  const redo = () => {
    const command = redoStack.at(-1);
    if (!command) return;
    setRedoStack((stack) => stack.slice(0, -1));
    setUndoStack((stack) => pushBounded(stack, command));
    persist(command.after);
  };
  return { record, undo, redo, canUndo: undoStack.length > 0, canRedo: redoStack.length > 0 };
}
