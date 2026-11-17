import type { Asset, CanvasNode, Note, Snapshot } from './engine_contract';

export type { Asset, CanvasNode, Note, Snapshot };

export const NOTE_PALETTE = {
  yellow: { label: '노랑', accent: '#e6bb55' },
  pink: { label: '분홍', accent: '#e2799e' },
  blue: { label: '파랑', accent: '#5396e8' },
  green: { label: '초록', accent: '#67b974' },
  purple: { label: '보라', accent: '#967ad4' },
} as const;

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
  const rows = markdown.match(/^\s*[-*+]\s+\[[ x/]]/gim) ?? [];
  return { total: rows.length, done: rows.filter((row) => /\[[xX]]/.test(row)).length };
}

export function projectPath(note: Note, notes: Note[]): string[] {
  const byId = new Map(notes.map((item) => [item.id, item]));
  const path: string[] = [];
  const seen = new Set<string>();
  let parent = note.canvas.parent;
  while (parent && byId.has(parent) && !seen.has(parent)) {
    seen.add(parent);
    const ancestor = byId.get(parent)!;
    path.unshift(titleOf(ancestor));
    parent = ancestor.canvas.parent;
  }
  return path;
}

export function visibleNoteOrder(notes: Note[], query: string, recent = false): Note[] {
  const normalized = query.trim().toLocaleLowerCase();
  const matches = notes.filter((note) => !normalized || note.text.toLocaleLowerCase().includes(normalized));
  if (normalized) {
    return matches.sort((a, b) => {
      const aTitle = titleOf(a).toLocaleLowerCase().includes(normalized);
      const bTitle = titleOf(b).toLocaleLowerCase().includes(normalized);
      return Number(bTitle) - Number(aTitle) || b.modified - a.modified;
    });
  }
  return matches.sort((a, b) => b.modified - a.modified);
}

export function makeSnapshot(notes: Note[], assets: Asset[] = []): Snapshot {
  return { notes, assets, revision: 0, savedRevision: 0 };
}
