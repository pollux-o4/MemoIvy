import type { Note } from '../../../shared/engine_contract';
import type { CanvasPoint } from '../model/align';
import { CARD_HEIGHT, CARD_WIDTH } from '../model/align';

export function CanvasLinks({ notes, allNotes, scope, positions, zoom, selected, sampleForLink }: {
  notes: Note[]; allNotes: Note[]; scope: string | null; positions: Record<string, CanvasPoint>; zoom: number; selected: string[];
  sampleForLink: (key: string) => { creating: boolean; pulsing: boolean; progress: number; travel: number; alpha: number };
}) {
  const noteIds = new Set(notes.map((note) => note.id));
  const byId = new Map(allNotes.map((note) => [note.id, note]));
  const representative = (id: string): string | undefined => {
    let note = byId.get(id);
    const seen = new Set<string>();
    while (note && note.canvas.parent !== scope && note.canvas.parent && !seen.has(note.id)) {
      seen.add(note.id); note = byId.get(note.canvas.parent);
    }
    return note?.canvas.parent === scope ? note.id : undefined;
  };
  const projected = new Set<string>();
  for (const note of allNotes) for (const target of note.canvas.next) {
    const from = representative(note.id); const to = representative(target);
    if (from && to && from !== to && (!noteIds.has(note.id) || !noteIds.has(target)) && !notes.find((item) => item.id === from)?.canvas.next.includes(to)) projected.add(`${from}:${to}`);
  }
  const paths: Array<{ fromId: string; toId: string; direct: boolean }> = notes.flatMap((note) =>
    note.canvas.next.filter((target) => noteIds.has(target)).map((target) => ({ fromId: note.id, toId: target, direct: true })),
  );
  paths.push(...[...projected].map((pair) => { const [fromId, toId] = pair.split(':'); return { fromId, toId, direct: false }; }));
  return <svg className="canvas-links" aria-hidden="true">
    {paths.map(({ fromId, toId, direct }) => {
      const from = positions[fromId]; const to = positions[toId]; if (!from || !to) return null;
      const sample = direct ? sampleForLink(`${fromId}:${toId}`) : { creating: false, pulsing: false, progress: 1, travel: 0, alpha: 1 };
      const horizontal = Math.abs(to.x - from.x) >= CARD_WIDTH + 24;
      const direction = horizontal ? (to.x > from.x ? 1 : -1) : (to.y >= from.y ? 1 : -1);
      const start: CanvasPoint = horizontal ? { x: from.x + (direction > 0 ? CARD_WIDTH : 0), y: from.y + CARD_HEIGHT / 2 } : { x: from.x + CARD_WIDTH / 2, y: from.y + (direction > 0 ? CARD_HEIGHT : 0) };
      const end: CanvasPoint = horizontal ? { x: to.x + (direction > 0 ? 0 : CARD_WIDTH), y: to.y + CARD_HEIGHT / 2 } : { x: to.x + CARD_WIDTH / 2, y: to.y + (direction > 0 ? 0 : CARD_HEIGHT) };
      const bend = Math.max(Math.abs((horizontal ? end.x - start.x : end.y - start.y) / 2), 40);
      const c1 = horizontal ? `${start.x + direction * bend} ${start.y}` : `${start.x} ${start.y + direction * bend}`;
      const c2 = horizontal ? `${end.x - direction * bend} ${end.y}` : `${end.x} ${end.y - direction * bend}`;
      const highlighted = selected.includes(fromId) || selected.includes(toId);
      return <g key={`${fromId}:${toId}:${direct}`}>
        <path d={`M ${start.x} ${start.y} C ${c1}, ${c2}, ${end.x} ${end.y}`}
        markerEnd="url(#arrow)" pathLength={1} stroke={highlighted ? 'var(--canvas-accent)' : undefined} strokeOpacity={highlighted ? .95 : undefined} strokeWidth={(highlighted ? 2.025 : 1.5) / Math.max(zoom, .45)}
        strokeDasharray={!direct ? '5 5' : sample.creating ? '1' : sample.pulsing ? '.12 .88' : undefined}
        strokeDashoffset={sample.creating ? 1 - sample.progress : sample.pulsing ? -sample.travel : undefined}
        opacity={sample.pulsing ? sample.alpha : 1}/>
        {direct && sample.pulsing && !sample.creating && sample.travel >= 1 && <rect x={to.x - 4} y={to.y - 4} width={CARD_WIDTH + 8} height={CARD_HEIGHT + 8} rx="18" fill="none" stroke={byId.get(toId)?.canvas.done ? 'var(--canvas-success)' : 'var(--canvas-accent)'} strokeWidth={2.5 / Math.max(zoom, .45)} opacity={sample.alpha * .85}/>}
      </g>;
    })}
    <defs><marker id="arrow" markerWidth="9" markerHeight="9" refX="8" refY="4.5" orient="auto"><path d="M0 0 L9 4.5 L0 9 Z" fill="currentColor"/></marker></defs>
  </svg>;
}
