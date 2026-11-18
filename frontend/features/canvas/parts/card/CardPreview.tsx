import { type CSSProperties, type Ref, type SyntheticEvent, useEffect } from 'react';
import { Disclosure } from '../../../../shared/Disclosure';
import { Icon } from '../../../../shared/Icon';
import { useCollapsible } from '../../../../shared/useCollapsible';
import type { PreviewItem } from '../../model/preview';

type Rows = readonly PreviewItem[];

/**
 * 본문 미리보기 줄 목록. 편집기와 같은 규칙으로 그린다: 일반 줄은 표식 칸 없이 왼쪽에서 시작하고,
 * 목록 줄은 깊이마다 한 칸(표식 칸 + 간격, CSS `--preview-gutter`)씩 들어간다. 카드·인스펙터가 함께 쓴다.
 */
export function PreviewList({ rows, className, id, ref }: { rows: Rows; className: string; id?: string; ref?: Ref<HTMLUListElement> }) {
  return <ul ref={ref} id={id} className={className}>
    {/* biome-ignore lint/suspicious/noArrayIndexKey: 미리보기 줄은 본문 순서 그대로의 위치값이라 순서가 곧 정체성이다. */}
    {rows.map((row, index) => <li className={`preview-${row.kind}`} style={{ '--depth': row.depth } as CSSProperties} key={index}>
      {row.kind === 'task' ? <span className={`preview-check${row.done ? ' checked' : ''}`} aria-hidden="true">{row.done ? <Icon name="check" size={12}/> : null}</span>
        : row.kind === 'number' ? <span className="preview-marker">{row.mark}</span>
          : row.kind === 'bullet' ? <span className="preview-marker">•</span> : null}
      <small className={row.done ? 'checked' : ''}>{row.text}</small>
    </li>)}
  </ul>;
}

/** 접힌 카드에 보일 줄 수. 넘치는 줄은 페이드로 가리고 펼침 버튼으로 연다. */
const COLLAPSED_ROWS = 3;

/** 앞의 COLLAPSED_ROWS 줄 바닥까지의 높이. */
const rowsHeight = (list: HTMLElement) => {
  const last = list.children[Math.min(COLLAPSED_ROWS, list.children.length) - 1] as HTMLElement | undefined;
  return last ? last.offsetTop - list.offsetTop + last.offsetHeight : 0;
};
const stop = (event: SyntheticEvent) => event.stopPropagation();

/** 카드 본문 미리보기. 메모·프로젝트 카드가 같이 쓴다. 넘치는 줄은 가리고 공통 펼침 버튼을 두며, 비었으면 안내 문구를 둔다. */
export function CardPreview({ id, label, rows, empty, onGrow }: { id: string; label: string; rows: Rows; empty: boolean; onGrow: (px: number) => void }) {
  const preview = useCollapsible<HTMLUListElement>(rowsHeight, [rows]);
  const { expanded } = preview;
  // 카드는 목록과 같은 곡선으로 늘어난 높이만큼 자라고 줄어든다(canvas.css `--preview-extra`).
  const grow = expanded ? preview.extra : 0;
  useEffect(() => onGrow(grow), [grow, onGrow]);
  if (!rows.length) return empty ? <p>내용을 적어 보세요.</p> : null;
  return <>
    <PreviewList ref={preview.ref} id={id} className={`canvas-checklist-preview ${preview.className}`} rows={rows}/>
    {/* biome-ignore lint/a11y/noStaticElementInteractions: 펼침 버튼 누름이 카드 끌기·더블클릭 열기로 번지지 않게 막기만 한다. */}
    {preview.needsReveal && <div className="canvas-preview-reveal" onPointerDown={stop} onDoubleClick={stop}>
      <Disclosure expanded={expanded} onToggle={preview.toggle} targetId={id} label={`${label} 본문`}/>
    </div>}
  </>;
}
