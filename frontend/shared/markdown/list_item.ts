/**
 * Markdown 목록 줄 문법의 단일 출처(GTK syntax list `line_mark`와 같은 규칙).
 * 편집기 표식·키 동작, 캔버스 미리보기, 할 일 집계가 모두 이것으로 줄을 읽는다.
 * 할 일은 `]` 뒤 공백이 있어야 한다. `- [ ]`만 있으면 "[ ]" 글자를 가진 글머리 항목이다.
 */
const LIST_LINE = /^(\s*)(?:(?:([-*+])\s+(?:\[([ xX/])\]\s+)?|(\d+)([.)])\s+))/;

export type TaskState = ' ' | 'x' | '/';
export type ListItem = { depth: number; contentIndent: number; marker: string; ordered: boolean; number?: number; delimiter?: string; task?: TaskState };

export function parseListItem(text: string): ParsedListItem | null {
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

export type FenceBoundary = { kind: 'open' | 'close'; delimiter: string; indent: string };
export type ListOwner = { item: ListItem; markerLine: number; sequence: number };
/** 한 줄의 블록 판정. item은 목록 표식(null: 일반 줄, undefined: 코드·인용 안이라 판정 제외). */
export type ParsedListItem = ListItem & { markerEnd: number };
export type BlockLine = { item: ParsedListItem | null | undefined; owner: ListOwner | null; code: boolean; fence?: FenceBoundary };

const FENCE_LINE = /^(\s*)(`{3,}|~{3,})([^\n]*)$/;
/** 여는 펜스: 정보 문자열은 공백 없는 한 단어(언어 이름)까지만. 문장을 이어 쓴 줄은 일반 글이다. */
const FENCE_OPENER = /^(\s*)(`{3,}|~{3,})([^\s`]*)\s*$/;

/** 여는 펜스 모양의 줄이면 경계를 돌려준다(입력 규칙과 스캐너의 공통 판정). */
export function fenceOpener(text: string): FenceBoundary | undefined {
  const match = FENCE_OPENER.exec(text);
  return match ? { kind: 'open', delimiter: match[2], indent: match[1] } : undefined;
}
const QUOTE_LINE = /^\s*>/;

/** 코드 펜스 짝짓기: 여는 펜스와 같은 문자·같거나 긴 길이의 빈 정보 줄만 닫는다(CommonMark). */
class FenceTracker {
  private open?: string;

  get inside(): boolean { return this.open !== undefined; }

  /** 이 줄이 펜스 경계면 경계를 돌려주고 상태를 바꾼다. `pending`인 줄은 아직 확정되지 않은 입력이라 블록을 열지 않는다. */
  advance(text: string, pending = false): FenceBoundary | undefined {
    if (this.open) {
      const match = FENCE_LINE.exec(text);
      if (!match) return undefined;
      const [, indent, delimiter, info] = match;
      if (delimiter[0] !== this.open[0] || delimiter.length < this.open.length || info.trim()) return undefined;
      this.open = undefined;
      return { kind: 'close', delimiter, indent };
    }
    const opener = pending ? undefined : fenceOpener(text);
    if (opener) this.open = opener.delimiter;
    return opener;
  }
}

/** 목록 소속 스택: 항목 내용 들여쓰기 이상의 이어쓰기 줄(이미지 포함)과 빈 줄 1개는 항목을 끊지 않는다. */
class ListOwners {
  private readonly stack: ListOwner[] = [];
  private sequence = 0;
  private blanks = 0;

  get current(): ListOwner | null { return this.stack[this.stack.length - 1] ?? null; }

  item(item: ListItem, line: number): void {
    this.blanks = 0;
    const { stack } = this;
    for (let top = this.current; top && top.item.depth > item.depth; top = this.current) stack.pop();
    const previous = this.current;
    const sibling = previous?.item.depth === item.depth;
    const sameList = sibling && previous.item.ordered === item.ordered && previous.item.delimiter === item.delimiter;
    if (sibling) stack.pop();
    stack.push({ item, markerLine: line, sequence: sameList ? previous.sequence : ++this.sequence });
  }

  text(indent: number, quote: boolean): void {
    this.blanks = 0;
    if (quote) { this.stack.length = 0; return; }
    for (let top = this.current; top && indent < top.item.contentIndent; top = this.current) this.stack.pop();
  }

  blank(): void { if (++this.blanks > 1) this.stack.length = 0; }
}

/**
 * 문서 순서로 줄마다 한 번 호출하는 블록 스캐너(줄당 O(목록 깊이), 상태 O(목록 깊이)).
 * `pendingLine`(1부터)은 Enter로 확정하기 전인 여는 펜스 줄이다. 입력 중에는 아래 블록과 짝짓지 않는다.
 */
export function createBlockScanner({ pendingLine }: { pendingLine?: number } = {}): (text: string) => BlockLine {
  const fence = new FenceTracker();
  const owners = new ListOwners();
  let line = 0;
  return (text) => {
    line++;
    const wasInside = fence.inside;
    const boundary = fence.advance(text, line === pendingLine);
    const code = wasInside || boundary !== undefined;
    const quote = QUOTE_LINE.test(text);
    const item = code || quote ? undefined : parseListItem(text);
    if (!text.trim()) owners.blank();
    else if (item) owners.item(item, line);
    else if (!wasInside) owners.text(text.length - text.trimStart().length, quote);
    return { item, owner: owners.current, code, fence: boundary };
  };
}

/** 1부터 센 `number`번째 줄의 판정. 앞줄만 읽는다. */
export function blockLineAt(lineText: (number: number) => string, number: number): BlockLine {
  const scan = createBlockScanner();
  let result: BlockLine = { item: null, owner: null, code: false };
  for (let current = 1; current <= number; current++) result = scan(lineText(current));
  return result;
}
