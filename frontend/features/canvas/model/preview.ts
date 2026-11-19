import { createListItemReader, type ListItem, taskBoxOffset, type TaskState } from '../../../shared/markdown/list_item';
import { plainText } from '../../../shared/model';

/** depth는 편집기 목록 들여쓰기(list_item.depth)와 같은 값이다. 일반 줄은 0. */
export type PreviewItem = { kind: 'task' | 'bullet' | 'number' | 'text'; text: string; depth: number; done?: boolean; mark?: string; task?: TaskState; sourceLine?: number };

/** `budget`을 빼면 모든 줄을 돌려준다(카드는 가림을 화면에서 처리한다). */
export function canvasPreview(markdown: string, budget = Number.POSITIVE_INFINITY): { title: PreviewItem; rows: PreviewItem[]; remaining: number } {
  const source = markdown.split('\n');
  const titleLine = source.findIndex((line) => line.trim());
  const readItem = createListItemReader();
  const heading = source[titleLine] ?? '';
  const title = previewLine(heading, readItem(heading));
  // 줄바꿈은 메모 편집기에서 보이는 그대로 한 줄씩 남긴다.
  const items = source.slice(titleLine + 1).flatMap((line, index) => line.trim()
    ? [previewLine(line, readItem(line), titleLine + 1 + index)] : []);
  return { title, rows: items.slice(0, budget), remaining: Math.max(0, items.length - budget) };
}

function previewLine(line: string, item: ListItem | null | undefined, sourceLine?: number): PreviewItem {
  const text = plainText(item ? line.slice(item.contentIndent) : line);
  if (!item) return { kind: 'text', text, depth: 0 };
  const { depth } = item;
  if (item.task) return { kind: 'task', text, depth, done: item.task === 'x', task: item.task, sourceLine };
  if (item.ordered) return { kind: 'number', text, depth, mark: `${item.number}${item.delimiter}` };
  return { kind: 'bullet', text, depth };
}

/** 원본 줄에서 체크 상태 한 글자만 교체한다. */
export function togglePreviewTask(markdown: string, sourceLine: number): string {
  const lines = markdown.split('\n');
  const line = lines[sourceLine];
  if (line === undefined) return markdown;
  const readItem = createListItemReader();
  for (let i = 0; i < sourceLine; i += 1) readItem(lines[i]);
  const item = readItem(line);
  if (!item?.task) return markdown;
  const at = taskBoxOffset(item) + 1;
  lines[sourceLine] = line.slice(0, at) + (item.task === 'x' ? ' ' : 'x') + line.slice(at + 1);
  return lines.join('\n');
}
