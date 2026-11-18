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
