import { plainText } from '../../../shared/model';

export type PreviewItem = { kind: 'task' | 'bullet' | 'number' | 'text'; text: string; done?: boolean; mark?: string; lines?: number };

export function canvasPreview(markdown: string, budget: number): { title: PreviewItem; rows: PreviewItem[]; remaining: number } {
  const source = markdown.split('\n');
  const titleLine = source.findIndex((line) => line.trim());
  const first = titleLine < 0 ? '' : source[titleLine];
  const titleMatch = /^\s*[-*+]\s+\[([ xX/])\]\s*(.*)$/.exec(first);
  const title: PreviewItem = titleMatch
    ? { kind: 'task', text: plainText(titleMatch[2]), done: titleMatch[1].toLowerCase() === 'x' }
    : /^\s*\d+[.)]\s+/.test(first) ? { kind: 'number', text: plainText(first), mark: /^\s*\d+[.)]/.exec(first)?.[0].trim() }
      : /^\s*[-*+]\s+/.test(first) ? { kind: 'bullet', text: plainText(first) } : { kind: 'text', text: plainText(first) };
  const items: PreviewItem[] = [];
  for (const line of source.slice(titleLine + 1)) {
    if (!line.trim()) continue;
    const task = /^\s*[-*+]\s+\[([ xX/])\]\s*(.*)$/.exec(line);
    const bullet = /^\s*[-*+]\s+(.*)$/.exec(line);
    const number = /^\s*(\d+[.)])\s+(.*)$/.exec(line);
    if (task) items.push({ kind: 'task', text: plainText(task[2]), done: task[1].toLowerCase() === 'x', lines: 1 });
    else if (bullet) items.push({ kind: 'bullet', text: plainText(bullet[1]), lines: 1 });
    else if (number) items.push({ kind: 'number', mark: number[1], text: plainText(number[2]), lines: 1 });
    else {
      const text = plainText(line);
      const previous = items.at(-1);
      if (previous?.kind === 'text') { previous.text += ` ${text}`; previous.lines = (previous.lines ?? 1) + 1; }
      else items.push({ kind: 'text', text, lines: 1 });
    }
  }
  return { title, rows: items.slice(0, budget), remaining: items.slice(budget).reduce((sum, item) => sum + (item.lines ?? 1), 0) };
}
