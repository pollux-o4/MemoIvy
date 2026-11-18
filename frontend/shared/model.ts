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

export function projectPath(note: Note, notes: Note[], title: (note: Note) => string = titleOf): string[] {
  const byId = new Map(notes.map((item) => [item.id, item]));
  const path: string[] = [];
  const seen = new Set<string>();
  let parent = note.canvas.parent;
  while (parent && byId.has(parent) && !seen.has(parent)) {
    seen.add(parent);
    const ancestor = byId.get(parent)!;
    path.unshift(title(ancestor));
    parent = ancestor.canvas.parent;
  }
  return path;
}


/** 프로젝트 하위(중첩 포함) 메모. 부모 순환은 끊는다. */
export function descendantsOf(projectId: string, notes: Note[]): Note[] {
  const byId = new Map(notes.map((item) => [item.id, item]));
  return notes.filter((item) => {
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
