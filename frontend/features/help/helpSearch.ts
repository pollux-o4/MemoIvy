import type { HelpTopic } from '../../shared/helpTopic';
import { CANVAS_ESSENTIALS, CANVAS_SHORTCUTS, type Essential, NOTE_ESSENTIALS, NOTE_SHORTCUTS, type ShortcutRow, type ShortcutSection } from '../../shared/shortcuts';

type Topic = { label: string; essentials: readonly Essential[]; sections: readonly ShortcutSection[] };
export const TOPICS: Record<HelpTopic, Topic> = {
  note: { label: '메모', essentials: NOTE_ESSENTIALS, sections: NOTE_SHORTCUTS },
  canvas: { label: '캔버스', essentials: CANVAS_ESSENTIALS, sections: CANVAS_SHORTCUTS },
};
export const TOPIC_OPTIONS = (Object.keys(TOPICS) as HelpTopic[]).map((value) => ({ value, label: TOPICS[value].label }));

const normalize = (text: string) => text.normalize('NFKC').toLowerCase();

const entries = TOPIC_OPTIONS.flatMap(({ value: topic, label }) => {
  const { sections, essentials } = TOPICS[topic];
  const rows = new Map<string, { section: string; row: ShortcutRow }>();
  for (const [section, shortcuts] of sections) {
    for (const row of shortcuts) rows.set(row[0], { section, row });
  }
  for (const [meaning, keys] of essentials) {
    const existing = rows.get(meaning);
    rows.set(meaning, {
      section: existing?.section ?? '기본',
      row: [meaning, existing?.row[1] ?? '', [...new Set([...keys, ...(existing?.row[2] ?? [])])]],
    });
  }
  return [...rows.values()].map(({ section, row: [meaning, syntax, keys] }) => ({
    topic, label, meaning, syntax, keys,
    match: [label, section, meaning, syntax, ...keys].map((field) => normalize(field).replace(/\s+/g, '')),
  }));
});

export function searchHelp(query: string) {
  const terms = normalize(query).trim().replace(/\s*\+\s*/g, '+').split(/\s+/).filter(Boolean);
  return terms.length ? entries.filter(({ match }) => terms.every((term) => match.some((field) => field.includes(term)))) : [];
}
