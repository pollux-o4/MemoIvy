import { blockLineAt, createBlockScanner, type ListItem, taskBoxOffset, type TaskState } from '../list_item';
import { plainText } from '../../model';

/** depth는 편집기 목록 들여쓰기(list_item.depth)와 같은 값이다. 일반 줄은 0. */
export type PreviewItem = { text: string; depth: number } & (
  | { kind: 'task'; task: TaskState; sourceLine?: number }
  | { kind: 'number'; mark: string }
  | { kind: 'heading' }
  | { kind: 'bullet' | 'text' }
);

/** `budget`을 빼면 모든 줄을 돌려준다(카드는 가림을 화면에서 처리한다). */
export function markdownPreview(markdown: string, budget = Number.POSITIVE_INFINITY): { title: PreviewItem; rows: PreviewItem[]; remaining: number } {
  const source = markdown.split('\n');
  const titleLine = source.findIndex((line) => line.trim());
  const scan = createBlockScanner();
  const heading = source[titleLine] ?? '';
  const title = previewLine(heading, scan(heading).item);
  // 줄바꿈은 메모 편집기에서 보이는 그대로 한 줄씩 남긴다.
  const items = source.slice(titleLine + 1).flatMap((line, index) => {
    const { item, owner, code } = scan(line);
    if (!line.trim()) return [];
    const row = previewLine(line, item, titleLine + 1 + index);
    if (!item && !code && owner) row.depth = owner.item.depth;
    return [row];
  });
  return { title, rows: items.slice(0, budget), remaining: Math.max(0, items.length - budget) };
}

function previewLine(line: string, item: ListItem | null | undefined, sourceLine?: number): PreviewItem {
  const text = plainText(item ? line.slice(item.contentIndent) : line);
  if (!item) return { kind: item === null && /^\s{0,3}#{1,6}\s+/.test(line) ? 'heading' : 'text', text, depth: 0 };
  const { depth } = item;
  if (item.task) return { kind: 'task', text, depth, task: item.task, sourceLine };
  if (item.ordered) return { kind: 'number', text, depth, mark: `${item.number}${item.delimiter}` };
  return { kind: 'bullet', text, depth };
}

/** 원본 줄에서 체크 상태 한 글자만 교체한다. */
export function togglePreviewTask(markdown: string, sourceLine: number): string {
  const lines = markdown.split('\n');
  const line = lines[sourceLine];
  if (line === undefined) return markdown;
  const { item } = blockLineAt((number) => lines[number - 1], sourceLine + 1);
  if (!item?.task) return markdown;
  const at = taskBoxOffset(item) + 1;
  lines[sourceLine] = line.slice(0, at) + (item.task === 'x' ? ' ' : 'x') + line.slice(at + 1);
  return lines.join('\n');
}


/** 앞선 체크 변경은 반영하되 줄 내용·위치가 달라진 원문은 덮지 않는다. */
export function rebasePreviewTask(original: string, current: string, sourceLine: number): string | undefined {
  if (original === current) return togglePreviewTask(current, sourceLine);
  const before = original.split('\n');
  const after = current.split('\n');
  if (before.length !== after.length) return undefined;
  const scanBefore = createBlockScanner();
  const scanAfter = createBlockScanner();
  let targetOffset: number | undefined;
  let targetTask: TaskState | undefined;
  for (let index = 0; index < before.length; index += 1) {
    const line = before[index];
    const nextLine = after[index];
    const oldItem = scanBefore(line).item;
    const newItem = scanAfter(nextLine).item;
    if (line !== nextLine) {
      if (!oldItem?.task || !newItem?.task) return undefined;
      const at = taskBoxOffset(oldItem) + 1;
      if (at !== taskBoxOffset(newItem) + 1 || line.slice(0, at) !== nextLine.slice(0, at) || line.slice(at + 1) !== nextLine.slice(at + 1)) return undefined;
    }
    if (index === sourceLine && newItem?.task) {
      targetOffset = taskBoxOffset(newItem) + 1;
      targetTask = newItem.task;
    }
  }
  if (targetOffset === undefined || targetTask === undefined) return current;
  const line = after[sourceLine];
  after[sourceLine] = line.slice(0, targetOffset) + (targetTask === 'x' ? ' ' : 'x') + line.slice(targetOffset + 1);
  return after.join('\n');
}
