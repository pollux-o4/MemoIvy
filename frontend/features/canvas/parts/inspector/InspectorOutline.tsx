import { type KeyboardEvent, useLayoutEffect, useRef, useState } from 'react';
import { Icon } from '../../../../shared/Icon';
import { Reveal } from '../../../../shared/Reveal';
import { ShortcutKeys } from '../../../../shared/ShortcutKeys';
import { OUTLINE_SHORTCUTS } from '../../../../shared/shortcuts';
import { directChildren } from '../../model/links/outline_tree';
import type { OutlineRow } from '../../model/links/types';
import { type Place, useOutlineDrag } from './useOutlineDrag';

/**
 * 3층 보조 · 가지 개요: 지금 화면 단계에서 선택 카드가 속한 가지를 들여쓰기 목록으로 보인다.
 * 순서: 끌어 놓기(행 위·아래 절반에 앞·뒤로) 또는 Alt+↑↓. 다른 부모 행 옆에 놓으면 그 부모 아래로 옮긴다.
 * 하위 프로젝트는 접힌 항목 하나. 선택 카드의 바로 아래 가지에는 연결 해제(×)를 둔다(연결 칸 칩을 대신한다). 항목에서 Tab은 들여쓰기, Shift+Tab은 내어쓰기(연결만 바뀌고 카드 위치는 그대로).
 */
export function InspectorOutline({ rows, current, alsoFrom = [], labelOf, onSelect, onIndent, onOutdent, onMove, onShift, onUnlink }: {
  rows: readonly OutlineRow[]; current: string; alsoFrom?: readonly string[]; labelOf: (id: string) => string; onUnlink?: (id: string) => void;
  onMove: (id: string, target: string, place: Place) => boolean; onShift: (id: string, step: -1 | 1) => boolean;
  onSelect: (id: string) => void; onIndent: (id: string) => boolean; onOutdent: (id: string) => boolean;
}) {
  const list = useRef<HTMLOListElement>(null);
  const keep = useRef<string | undefined>(undefined);
  const [blocked, setBlocked] = useState<string | undefined>(undefined);
  const [help, setHelp] = useState(false);
  const children = directChildren(rows, current);
  const drag = useOutlineDrag({ labelOf, onMove, onMoved: (id) => { keep.current = id; } });
  // 들여쓰기로 목록이 다시 그려져도 같은 항목에 포커스를 둔다.
  useLayoutEffect(() => {
    if (!keep.current) return;
    list.current?.querySelector<HTMLButtonElement>(`[data-row="${CSS.escape(keep.current)}"]`)?.focus();
    keep.current = undefined;
  });
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>, id: string, index: number) => {
    if (event.key === 'Tab') {
      // 개요 편집기 관례: 바꿀 수 없어도 포커스는 그 자리에 두고 이유만 알린다. 목록 밖으로는 화살표·클릭으로 나간다.
      event.preventDefault(); event.stopPropagation();
      if (event.shiftKey ? onOutdent(id) : onIndent(id)) { keep.current = id; setBlocked(undefined); return; }
      setBlocked(event.shiftKey ? '이미 맨 위 단계예요' : '위에 같은 단계 항목이 없어요');
      return;
    }
    const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
    if (!step) return;
    event.preventDefault(); event.stopPropagation();
    if (event.altKey) {
      if (onShift(id, step)) { keep.current = id; setBlocked(undefined); } else setBlocked(step < 0 ? '위로 옮길 같은 단계 항목이 없어요' : '아래로 옮길 같은 단계 항목이 없어요');
      return;
    }
    list.current?.querySelectorAll<HTMLButtonElement>('[data-row]')[index + step]?.focus();
  };
  return <section className="inspector-group" aria-label="가지 개요">
    <header><b>가지 개요</b>
      <button type="button" className="inspector-help" aria-label="가지 개요 단축키" title="가지 개요 단축키" aria-expanded={help} aria-controls="outline-shortcuts" onClick={() => setHelp(!help)}><Icon name="keyboard" size={14}/></button>
    </header>
    <Reveal open={help}><dl id="outline-shortcuts" className="outline-shortcuts">{OUTLINE_SHORTCUTS.map(([what, , keys]) => <div key={what}><dt><ShortcutKeys keys={keys}/></dt><dd>{what}</dd></div>)}</dl></Reveal>
    <ol ref={list} className="inspector-outline">
      {rows.map((row, index) => <li key={row.id} style={{ '--depth': row.depth } as React.CSSProperties} {...drag.target(row)}>
        <button data-row={row.id} {...drag.source(row)} aria-current={row.id === current ? 'true' : undefined} aria-keyshortcuts="Tab Shift+Tab Alt+ArrowUp Alt+ArrowDown"
          title={row.project ? `${labelOf(row.id)} · 프로젝트` : labelOf(row.id)}
          onClick={() => onSelect(row.id)} onBlur={() => setBlocked(undefined)} onKeyDown={(event) => onKeyDown(event, row.id, index)}>
          <span className={`outline-bullet${row.hasChildren ? ' has-children' : ''}`} aria-hidden="true">{row.project && <Icon name="project" size={11}/>}</span>
          <span className="outline-label">{labelOf(row.id)}</span>
          {row.id === current && alsoFrom.length > 0 && <span className="outline-merge" title={`${alsoFrom.length + 1}곳에서 이어짐`}>+{alsoFrom.length}</span>}
        </button>
        {onUnlink && children.has(row.id) && <button className="inspector-disconnect" aria-label={`${labelOf(row.id)} 연결 해제`} title="이 연결 해제 · 메모는 유지" onClick={() => onUnlink(row.id)}><Icon name="close" size={11}/></button>}
      </li>)}
    </ol>
    {alsoFrom.length > 0 && <div className="inspector-chips outline-also" role="group" aria-label="다른 곳에서도 이어짐">
      <small title="이 메모로 들어오는 다른 연결. 개요는 한 갈래만 나무로 보여 줍니다.">다른 곳에서도 이어짐</small>
      {alsoFrom.map((id) => <button className="inspector-chip" key={id} title={labelOf(id)} onClick={() => onSelect(id)}>{labelOf(id)}</button>)}
    </div>}
    {/* 바꿀 수 없는 이유는 목록 아래에 잠깐 알린다(머리글을 비워 제목이 줄바꿈되지 않게). */}
    <small className="inspector-hint outline-status" role="status">{blocked ?? ''}</small>
  </section>;
}
