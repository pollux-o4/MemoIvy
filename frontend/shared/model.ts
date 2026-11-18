import type { CanvasNode, Note } from './engine_contract';
import { parseListItem } from './markdown/list_item';


/** 메모 색 이름의 단일 출처. 실제 색값은 tokens.css의 --note-* / --dot-* 가 가진다. */
const NOTE_PALETTE = {
  yellow: { label: '노랑', tone: 'var(--note-yellow)', accent: 'var(--dot-yellow)' },
  pink: { label: '분홍', tone: 'var(--note-pink)', accent: 'var(--dot-pink)' },
  blue: { label: '파랑', tone: 'var(--note-blue)', accent: 'var(--dot-blue)' },
  green: { label: '초록', tone: 'var(--note-green)', accent: 'var(--dot-green)' },
  purple: { label: '보라', tone: 'var(--note-purple)', accent: 'var(--dot-purple)' },
} as const;
export type NoteColor = keyof typeof NOTE_PALETTE;
export const NOTE_COLORS = Object.keys(NOTE_PALETTE) as NoteColor[];
export const noteColor = (color: string) => NOTE_PALETTE[color as NoteColor] ?? NOTE_PALETTE.yellow;

/** 체크리스트 줄 파싱. 줄 문법은 markdown/list_item이 단일 출처다(목록 개수·진행률 공용). */
function checklistRows(markdown: string): Array<{ done: boolean; text: string }> {
  return markdown.split('\n').flatMap((line) => {
    const item = parseListItem(line);
    return item?.task ? [{ done: item.task === 'x', text: line.slice(item.contentIndent) }] : [];
  });
}

/** 메모 본문 = 첫 줄 제목 + 나머지 소개. 제목 줄의 머리 표식(#, >)은 보존한다. 제목·소개 편집의 단일 출처. */
export function splitTitle(text: string): { marker: string; title: string; body: string } {
  const lines = text.split('\n');
  const index = lines.findIndex((line) => line.trim());
  if (index < 0) return { marker: '', title: '', body: '' };
  const match = /^(\s*(?:#{1,6}\s+|>\s+)?)(.*)$/.exec(lines[index]) as RegExpExecArray;
  return { marker: match[1], title: match[2].trim(), body: lines.slice(index + 1).join('\n').replace(/^\n+/, '') };
}
export function joinTitle(parts: { marker?: string; title: string; body: string }): string {
  const head = `${parts.marker ?? ''}${parts.title.trim()}`;
  // 끝의 빈 줄만 지운다. 마지막 줄의 공백은 남겨야 쓰는 중인 빈 할 일(`- [ ] `)이 글머리로 바뀌지 않는다(메모 편집기와 같은 원문).
  const body = parts.body.replace(/(\n[ \t]*)+$/, '');
  if (!body.trim()) return head;
  return `${head}\n\n${body}`;
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

/** 카드·드로어가 보이는 할 일 합계: 프로젝트는 하위까지, 메모는 자기 본문만. */
export function noteTasks(note: Note, descendants: readonly Note[]): { total: number; done: number } {
  return note.canvas.project ? projectTasks(note, descendants) : checklist(note.text);
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
