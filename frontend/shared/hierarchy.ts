import type { Note } from './engine_contract';

/** 부모 → 바로 아래 메모. 진행·개수·부채가 모두 이 한 단계 기준을 쓴다. */
export type ChildIndex = (parentId: string) => readonly Note[];

const NO_NOTES: readonly Note[] = [];

export function createChildIndex(notes: readonly Note[]): ChildIndex {
  const children = new Map<string, Note[]>();
  for (const item of notes) {
    const parent = item.canvas.parent;
    if (!parent) continue;
    const list = children.get(parent);
    if (list) list.push(item); else children.set(parent, [item]);
  }
  return (parentId) => children.get(parentId) ?? NO_NOTES;
}

/** 하위 메모 개수 문구의 단일 출처. */
export function childCountLabel(count: number): string {
  return `메모 ${count}개`;
}
