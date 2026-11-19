import type { OutlineRow } from './types';

export interface NextChange { id: string; next: string[] }

function findPrecedingByDepth(rows: readonly OutlineRow[], beforeIdx: number, depth: number): number {
  for (let i = beforeIdx - 1; i >= 0; i -= 1) {
    if (rows[i].depth === depth) return i;
  }
  return -1;
}

function findPrecedingSibling(rows: readonly OutlineRow[], beforeIdx: number, depth: number): number {
  for (let i = beforeIdx - 1; i >= 0; i -= 1) {
    if (rows[i].depth < depth) return -1;
    if (rows[i].depth === depth) return i;
  }
  return -1;
}

/** 행을 바로 위 형제의 마지막 자식으로 들여쓴 next 변경 목록을 반환한다. */
export function indentRow(
  rows: readonly OutlineRow[],
  id: string,
  nextOf: (id: string) => readonly string[],
): NextChange[] {
  const idx = rows.findIndex((r) => r.id === id);
  if (idx < 0) return [];
  const depth = rows[idx].depth;
  if (depth <= 0) return [];

  const siblingIdx = findPrecedingSibling(rows, idx, depth);
  if (siblingIdx < 0) return [];
  const parentIdx = findPrecedingByDepth(rows, idx, depth - 1);
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
  const idx = rows.findIndex((r) => r.id === id);
  if (idx < 0) return [];
  const depth = rows[idx].depth;
  if (depth < 2) return [];

  const parentIdx = findPrecedingByDepth(rows, idx, depth - 1);
  if (parentIdx < 0) return [];
  const grandparentIdx = findPrecedingByDepth(rows, parentIdx, depth - 2);
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
