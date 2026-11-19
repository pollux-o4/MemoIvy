import { useMemo } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import { type CanvasPoint, canvasSlot, cardRect } from '../../model/align';

/**
 * 현재 범위 메모에서 화면 배치를 끌어낸다(단일 출처). 저장 위치 위에 끌기 초안을 덮는다.
 * 저장 위치 객체는 메모가 바뀔 때만 새로 만든다. 끌기 중에는 움직인 카드의 위치 참조만 바뀐다.
 */
export function useCanvasLayout(notes: Note[], draftPositions: Record<string, CanvasPoint>, selected: string[]) {
  const noteById = useMemo(() => new Map(notes.map((note) => [note.id, note])), [notes]);
  const savedPositions = useMemo(() => Object.fromEntries(notes.map((note, index) =>
    [note.id, note.canvas.position ? { x: note.canvas.position[0], y: note.canvas.position[1] } : canvasSlot(index)])), [notes]);
  const positions = useMemo(() => Object.fromEntries(notes.map((note) => [note.id, draftPositions[note.id] ?? savedPositions[note.id]])), [notes, draftPositions, savedPositions]);
  const cardRects = useMemo(() => notes.map((note) => cardRect(positions[note.id])), [notes, positions]);
  const selectedRects = useMemo(() => selected.flatMap((id) => positions[id] ? [cardRect(positions[id])] : []), [selected, positions]);
  return { noteById, positions, cardRects, selectedRects };
}
