import type { Note } from '../../../shared/engine_contract';

export const MAX_PROJECT_DEPTH = 3;

export function ancestry(id: string | null | undefined, notes: Note[]): Note[] {
  const byId = new Map(notes.map((note) => [note.id, note]));
  const chain: Note[] = [];
  const seen = new Set<string>();
  let cursor = id ? byId.get(id) : undefined;
  while (cursor && !seen.has(cursor.id)) {
    seen.add(cursor.id);
    chain.push(cursor);
    cursor = cursor.canvas.parent ? byId.get(cursor.canvas.parent) : undefined;
  }
  return chain.reverse();
}

export function scopeDepth(scopeId: string | null, notes: Note[]): number {
  return ancestry(scopeId, notes).length;
}

export function canMoveInto(targetId: string, movingIds: string[], notes: Note[]): boolean {
  const chain = ancestry(targetId, notes);
  return chain.length < MAX_PROJECT_DEPTH && !chain.some((note) => movingIds.includes(note.id));
}

export function scopeShortcuts(pathIds: Set<string>, recent: string[], notes: Note[], limit = 5): string[] {
  const pinned = notes.filter((note) => note.canvas.project && note.canvas.pinned).map((note) => note.id);
  return [...pinned, ...recent]
    .filter((id, index, ids) => !pathIds.has(id) && ids.indexOf(id) === index)
    .slice(0, limit);
}
