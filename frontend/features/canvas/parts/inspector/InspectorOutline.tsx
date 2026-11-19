import { type KeyboardEvent, useLayoutEffect, useRef } from 'react';
import { Icon } from '../../../../shared/Icon';
import type { OutlineRow } from '../../model/links/types';

/**
 * 3층 보조 · 가지 개요: 지금 화면 단계에서 선택 카드가 속한 가지를 들여쓰기 목록으로 보인다.
 * 하위 프로젝트는 접힌 항목 하나. 항목에서 Tab은 들여쓰기, Shift+Tab은 내어쓰기(연결만 바뀌고 카드 위치는 그대로).
 */
export function InspectorOutline({ rows, current, labelOf, onSelect, onIndent, onOutdent }: {
  rows: readonly OutlineRow[]; current: string; labelOf: (id: string) => string;
  onSelect: (id: string) => void; onIndent: (id: string) => boolean; onOutdent: (id: string) => boolean;
}) {
  const list = useRef<HTMLOListElement>(null);
  const keep = useRef<string | undefined>(undefined);
  // 들여쓰기로 목록이 다시 그려져도 같은 항목에 포커스를 둔다.
  useLayoutEffect(() => {
    if (!keep.current) return;
    list.current?.querySelector<HTMLButtonElement>(`[data-row="${CSS.escape(keep.current)}"]`)?.focus();
    keep.current = undefined;
  });
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>, id: string, index: number) => {
    if (event.key === 'Tab') {
      // 바꿀 수 없으면 기본 포커스 이동에 맡긴다.
      if (!(event.shiftKey ? onOutdent(id) : onIndent(id))) return;
      event.preventDefault(); event.stopPropagation(); keep.current = id;
      return;
    }
    const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    event.preventDefault(); event.stopPropagation();
    list.current?.querySelectorAll<HTMLButtonElement>('[data-row]')[index + step]?.focus();
  };
  return <section className="inspector-group" aria-label="가지 개요">
    <header><b>가지 개요</b><small className="inspector-hint">Tab 들여쓰기</small></header>
    <ol ref={list} className="inspector-outline">
      {rows.map((row, index) => <li key={row.id} style={{ '--depth': row.depth } as React.CSSProperties}>
        <button data-row={row.id} aria-current={row.id === current ? 'true' : undefined}
          title={row.project ? `${labelOf(row.id)} · 프로젝트` : labelOf(row.id)}
          onClick={() => onSelect(row.id)} onKeyDown={(event) => onKeyDown(event, row.id, index)}>
          <span className={`outline-bullet${row.hasChildren ? ' has-children' : ''}`} aria-hidden="true">{row.project && <Icon name="project" size={11}/>}</span>
          <span className="outline-label">{labelOf(row.id)}</span>
        </button>
      </li>)}
    </ol>
  </section>;
}
