import { parseListItem } from '../../../shared/markdown/list_item';
import { plainText } from '../../../shared/model';

export type PreviewItem = { kind: 'task' | 'bullet' | 'number' | 'text'; text: string; done?: boolean; mark?: string };

export function canvasPreview(markdown: string, budget: number): { title: PreviewItem; rows: PreviewItem[]; remaining: number } {
  const source = markdown.split('\n');
  const titleLine = source.findIndex((line) => line.trim());
  const title = previewLine(titleLine < 0 ? '' : source[titleLine]);
  // 줄바꿈은 메모 편집기에서 보이는 그대로 한 줄씩 남긴다.
  const items = source.slice(titleLine + 1).filter((line) => line.trim()).map(previewLine);
  return { title, rows: items.slice(0, budget), remaining: Math.max(0, items.length - budget) };
}

function previewLine(line: string): PreviewItem {
  const item = parseListItem(line);
  const text = plainText(item ? line.slice(item.contentIndent) : line);
  if (!item) return { kind: 'text', text };
  if (item.task) return { kind: 'task', text, done: item.task === 'x' };
  if (item.ordered) return { kind: 'number', text, mark: `${item.number}${item.delimiter}` };
  return { kind: 'bullet', text };
}
