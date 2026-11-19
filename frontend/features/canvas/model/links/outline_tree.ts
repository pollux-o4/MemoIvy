import type { OutlineRow } from './types';

/** 가지 개요 행(깊이 순 나열)에서 나무 관계를 읽는 질의. 편집(outline_edit)과 화면(InspectorOutline)이 같이 쓴다. */

export function findPrecedingSibling(rows: readonly OutlineRow[], beforeIdx: number, depth: number): number {
  for (let i = beforeIdx - 1; i >= 0; i -= 1) {
    if (rows[i].depth < depth) return -1;
    if (rows[i].depth === depth) return i;
  }
  return -1;
}

/** 행의 부모 인덱스: 위로 올라가며 처음 만나는 한 단계 얕은 행. */
export function parentIndex(rows: readonly OutlineRow[], idx: number): number {
  const depth = rows[idx].depth;
  for (let i = idx - 1; i >= 0; i -= 1) {
    if (rows[i].depth === depth - 1) return i;
  }
  return -1;
}

/** idx 행과 그 아래 가지(더 깊은 연속 행)의 끝 인덱스(포함하지 않음). */
export function subtreeEnd(rows: readonly OutlineRow[], idx: number): number {
  let end = idx + 1;
  while (end < rows.length && rows[end].depth > rows[idx].depth) end += 1;
  return end;
}

/** 개요에서 행의 나무 부모(위로 처음 만나는 한 단계 얕은 행). 뿌리면 undefined. */
export function outlineParent(rows: readonly OutlineRow[], id: string): string | undefined {
  const at = rows.findIndex((row) => row.id === id);
  return at < 0 ? undefined : rows[parentIndex(rows, at)]?.id;
}

/** 선택 행 바로 아래 단계의 행들: 다음 행부터 같은·얕은 단계가 나올 때까지 중 한 단계 깊은 것. */
export function directChildren(rows: readonly OutlineRow[], current: string): Set<string> {
  const at = rows.findIndex((row) => row.id === current);
  const children = new Set<string>();
  if (at < 0) return children;
  for (let i = at + 1, end = subtreeEnd(rows, at); i < end; i += 1) {
    const row = rows[i];
    if (row.depth === rows[at].depth + 1) children.add(row.id);
  }
  return children;
}
