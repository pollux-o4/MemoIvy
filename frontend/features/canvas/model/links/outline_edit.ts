import { findPrecedingSibling, parentIndex, subtreeEnd } from './outline_tree';
import type { OutlineRow } from './types';

export interface NextChange { id: string; next: string[] }

function editableRowIndex(rows: readonly OutlineRow[], id: string, minimumDepth: number): number {
  const idx = rows.findIndex((r) => r.id === id);
  return idx < 0 || rows[idx].depth < minimumDepth ? -1 : idx;
}

/** 행을 바로 위 형제의 마지막 자식으로 들여쓴 next 변경 목록을 반환한다. */
export function indentRow(
  rows: readonly OutlineRow[],
  id: string,
  nextOf: (id: string) => readonly string[],
): NextChange[] {
  const idx = editableRowIndex(rows, id, 1);
  if (idx < 0) return [];
  const depth = rows[idx].depth;

  const siblingIdx = findPrecedingSibling(rows, idx, depth);
  if (siblingIdx < 0) return [];
  const parentIdx = parentIndex(rows, idx);
  if (parentIdx < 0) return [];

  const parent = rows[parentIdx];
  const sibling = rows[siblingIdx];
  const parentNext = nextOf(parent.id).filter((x) => x !== id);
  const siblingBase = nextOf(sibling.id);
  const siblingNext = siblingBase.includes(id) ? [...siblingBase] : [...siblingBase, id];

  return [
    { id: parent.id, next: parentNext },
    { id: sibling.id, next: siblingNext },
  ];
}

/** 행을 조부모의 next에서 부모 바로 뒤로 내어쓴 next 변경 목록을 반환한다. */
export function outdentRow(
  rows: readonly OutlineRow[],
  id: string,
  nextOf: (id: string) => readonly string[],
): NextChange[] {
  const idx = editableRowIndex(rows, id, 2);
  if (idx < 0) return [];

  const parentIdx = parentIndex(rows, idx);
  if (parentIdx < 0) return [];
  const grandparentIdx = parentIndex(rows, parentIdx);
  if (grandparentIdx < 0) return [];

  const parent = rows[parentIdx];
  const grandparent = rows[grandparentIdx];
  const parentNext = nextOf(parent.id).filter((x) => x !== id);
  const gpBase = nextOf(grandparent.id);
  let grandparentNext: string[];
  if (gpBase.includes(id)) {
    grandparentNext = [...gpBase];
  } else {
    const parentPos = gpBase.indexOf(parent.id);
    grandparentNext = parentPos < 0
      ? [...gpBase, id]
      : [...gpBase.slice(0, parentPos + 1), id, ...gpBase.slice(parentPos + 1)];
  }

  return [
    { id: parent.id, next: parentNext },
    { id: grandparent.id, next: grandparentNext },
  ];
}

/**
 * 행(과 그 가지)을 target 행의 앞·뒤 형제로 옮긴 next 변경 목록. 끌어 놓기·Alt+↑↓가 쓴다.
 * 뿌리 옆, 자기 가지 안으로는 옮기지 않는다. 같은 부모 안이면 순서만, 다른 부모면 연결도 옮긴다.
 */
export function moveRow(
  rows: readonly OutlineRow[],
  id: string,
  targetId: string,
  place: 'before' | 'after',
  nextOf: (id: string) => readonly string[],
): NextChange[] {
  const idx = rows.findIndex((r) => r.id === id);
  const targetIdx = rows.findIndex((r) => r.id === targetId);
  if (idx < 0 || targetIdx < 0 || idx === targetIdx || rows[idx].depth <= 0 || rows[targetIdx].depth <= 0) return [];
  if (targetIdx > idx && targetIdx < subtreeEnd(rows, idx)) return [];
  const from = rows[parentIndex(rows, idx)]?.id;
  const to = rows[parentIndex(rows, targetIdx)]?.id;
  if (!from || !to) return [];
  const base = nextOf(to).filter((x) => x !== id);
  const at = base.indexOf(targetId);
  if (at < 0) return [];
  base.splice(place === 'before' ? at : at + 1, 0, id);
  if (from === to) return base.join() === nextOf(to).join() ? [] : [{ id: to, next: base }];
  return [{ id: from, next: nextOf(from).filter((x) => x !== id) }, { id: to, next: base }];
}

/** 같은 부모 안에서 바로 위(-1)·아래(+1) 형제와 자리를 바꾼다. 형제가 없으면 빈 목록. */
export function shiftRow(rows: readonly OutlineRow[], id: string, step: -1 | 1, nextOf: (id: string) => readonly string[]): NextChange[] {
  const idx = rows.findIndex((r) => r.id === id);
  if (idx < 0) return [];
  const depth = rows[idx].depth;
  if (step < 0) {
    const sibling = findPrecedingSibling(rows, idx, depth);
    return sibling < 0 ? [] : moveRow(rows, id, rows[sibling].id, 'before', nextOf);
  }
  const after = subtreeEnd(rows, idx);
  return after < rows.length && rows[after].depth === depth ? moveRow(rows, id, rows[after].id, 'after', nextOf) : [];
}
