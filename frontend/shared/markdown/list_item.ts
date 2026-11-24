/**
 * Markdown 목록 줄 문법의 단일 출처(GTK syntax list `line_mark`와 같은 규칙).
 * 편집기 표식·키 동작, 캔버스 미리보기, 할 일 집계가 모두 이것으로 줄을 읽는다.
 * 할 일은 `]` 뒤 공백이 있어야 한다. `- [ ]`만 있으면 "[ ]" 글자를 가진 글머리 항목이다.
 */
const LIST_LINE = /^(\s*)(?:(?:([-*+])\s+(?:\[([ xX/])\]\s+)?|(\d+)([.)])\s+))/;

export type TaskState = ' ' | 'x' | '/';
export type ListItem = { depth: number; contentIndent: number; marker: string; ordered: boolean; number?: number; delimiter?: string; task?: TaskState };

export function parseListItem(text: string): (ListItem & { markerEnd: number }) | null {
  const match = LIST_LINE.exec(text);
  if (!match) return null;
  const marker = match[0];
  return {
    depth: Math.min(Math.floor(match[1].replace(/\t/g, '  ').length / 2), 8),
    contentIndent: marker.length,
    marker,
    ordered: match[4] !== undefined,
    number: match[4] ? Number(match[4]) : undefined,
    delimiter: match[5],
    markerEnd: marker.length,
    task: match[3]?.toLowerCase() as TaskState | undefined,
  };
}

/** 체크 표식의 여는 대괄호 위치. 본문에 있는 대괄호는 제외한다. */
export const taskBoxOffset = (item: ListItem): number => item.marker.lastIndexOf('[');

/** 문서 순서로 한 번 읽는다. 호출 결과는 표식, context는 해당 줄의 블록·항목 소속이다. */
export function createListItemReader() {
  type Owner = { item: ListItem; markerLine: number; sequence: number };
  type Context = {
    owner: Owner | null;
    code: boolean;
    fence?: { kind: 'open' | 'close'; delimiter: string; indent: string };
  };
  let fence: string | undefined;
  let line = 0;
  let sequence = 0;
  let blanks = 0;
  const owners: Owner[] = [];
  const read = (text: string) => {
    line++;
    const indent = text.length - text.trimStart().length;
    const blank = !text.trim();
    const delimiter = /^(\s*)(`{3,}|~{3,})([^\n]*)$/.exec(text);
    const opening = delimiter && !(delimiter[2][0] === '`' && delimiter[3].includes('`'));
    const inside = fence !== undefined;
    let boundary: Context['fence'];
    if (fence) {
      if (delimiter && delimiter[2][0] === fence[0] && delimiter[2].length >= fence.length && !delimiter[3].trim()) {
        boundary = { kind: 'close', delimiter: delimiter[2], indent: delimiter[1] };
        fence = undefined;
      }
    } else if (opening) {
      fence = delimiter[2];
      boundary = { kind: 'open', delimiter: fence, indent: delimiter[1] };
    }
    const code = inside || !!boundary;
    const item = code || /^\s*>/.test(text) ? undefined : parseListItem(text);
    if (blank) {
      if (++blanks > 1) owners.length = 0;
    } else {
      blanks = 0;
      if (item) {
        while (owners.length && owners[owners.length - 1].item.depth > item.depth) owners.pop();
        const previous = owners[owners.length - 1];
        const same = previous && previous.item.depth === item.depth;
        const group = same && previous.item.ordered === item.ordered && previous.item.delimiter === item.delimiter
          ? previous.sequence : ++sequence;
        if (same) owners.pop();
        owners.push({ item, markerLine: line, sequence: group });
      } else if (!inside) {
        while (owners.length && indent < owners[owners.length - 1].item.contentIndent) owners.pop();
        if (/^\s*>/.test(text)) owners.length = 0;
      }
    }
    read.context = { owner: owners[owners.length - 1] ?? null, code, fence: boundary };
    return item;
  };
  read.context = { owner: null, code: false } as Context;
  return read;
}
