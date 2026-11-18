import type { CanvasNode, Note } from './engine_contract';


/** 메모 색 이름의 단일 출처. 실제 색값은 tokens.css의 --note-* / --dot-* 가 가진다. */
const NOTE_PALETTE = {
  yellow: { label: '노랑', tone: 'var(--note-yellow)', accent: 'var(--dot-yellow)' },
  pink: { label: '분홍', tone: 'var(--note-pink)', accent: 'var(--dot-pink)' },
  blue: { label: '파랑', tone: 'var(--note-blue)', accent: 'var(--dot-blue)' },
  green: { label: '초록', tone: 'var(--note-green)', accent: 'var(--dot-green)' },
  purple: { label: '보라', tone: 'var(--note-purple)', accent: 'var(--dot-purple)' },
} as const;
export type NoteColor = keyof typeof NOTE_PALETTE;
export const noteColor = (color: string) => NOTE_PALETTE[color as NoteColor] ?? NOTE_PALETTE.yellow;

const checklistRow = /^\s*[-*+]\s+\[([ xX/])\]\s*(.*)$/;
/** 체크리스트 줄 파싱의 단일 출처(목록 개수·캔버스 미리보기 공용). */
function checklistRows(markdown: string): Array<{ done: boolean; text: string }> {
  return markdown.split('\n').flatMap((line) => { const match = checklistRow.exec(line); return match ? [{ done: match[1].toLowerCase() === 'x', text: match[2] }] : []; });
}

export const emptyCanvas = (): CanvasNode => ({ position: null, next: [], done: false });

export function titleOf(note: Note): string {
  const line = note.text.split('\n').map((item) => item.trim()).find(Boolean) ?? '';
  return line.replace(/^#{1,6}\s+/, '').replace(/^>\s+/, '').trim() || '새 메모';
}

export function plainText(markdown: string): string {
  return markdown
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
    .replace(/^\s{0,3}(?:#{1,6}\s+|[-*+]\s+|\d+[.)]\s+|>\s*)/gm, '')
    .replace(/\[[ x/]]\s*/gi, '')
    .replace(/[`*_~]/g, '')
    .replace(/<[^>]+>/g, '')
    .trim();
}

export function checklist(markdown: string): { total: number; done: number } {
  const rows = checklistRows(markdown);
  return { total: rows.length, done: rows.filter((row) => row.done).length };
}

/** 프로젝트 할 일 합계: 프로젝트 본문과 모든 하위 메모의 체크리스트를 더한다. 목록·캔버스가 함께 쓴다. */
export function projectTasks(project: Note, descendants: readonly Note[]): { total: number; done: number } {
  return [project, ...descendants].reduce((sum, item) => { const value = checklist(item.text); return { total: sum.total + value.total, done: sum.done + value.done }; }, { total: 0, done: 0 });
}

export function projectPath(note: Note, notes: Note[], title: (note: Note) => string = titleOf): string[] {
  const byId = new Map(notes.map((item) => [item.id, item]));
  const path: string[] = [];
  const seen = new Set<string>();
  let parent = note.canvas.parent;
  for (let ancestor = parent ? byId.get(parent) : undefined; ancestor && !seen.has(ancestor.id); ancestor = parent ? byId.get(parent) : undefined) {
    seen.add(ancestor.id);
    path.unshift(title(ancestor));
    parent = ancestor.canvas.parent;
  }
  return path;
}

/** 프로젝트 하위(중첩 포함) 메모. 부모 순환은 끊는다. */
export function descendantsOf(projectId: string, notes: Note[]): Note[] {
  const byId = new Map(notes.map((item) => [item.id, item]));
  return notes.filter((item) => {
    if (item.id === projectId) return false;
    let parent = item.canvas.parent;
    const seen = new Set<string>();
    while (parent && !seen.has(parent)) {
      if (parent === projectId) return true;
      seen.add(parent);
      parent = byId.get(parent)?.canvas.parent ?? null;
    }
    return false;
  });
}

/** 프로젝트 종이 더미 장수: 프로젝트는 비어도 1장, 하위 메모가 2개 이상이면 2장. 목록·캔버스가 함께 쓴다. */
export function projectSheets(childCount: number): 1 | 2 {
  return childCount > 1 ? 2 : 1;
}
