import { type SyntheticEvent, useEffect } from 'react';
import { Disclosure } from '../../../../shared/Disclosure';
import { useCollapsible } from '../../../../shared/useCollapsible';
import type { PreviewItem } from '../../../../shared/markdown/preview/model';
import { PreviewList, previewRowsHeight } from '../../../../shared/markdown/preview/PreviewList';

type Rows = readonly PreviewItem[];

/** 접힌 카드에 보일 줄 수. 넘치는 줄은 페이드로 가리고 펼침 버튼으로 연다. */
const COLLAPSED_ROWS = 3;

const stop = (event: SyntheticEvent) => event.stopPropagation();

/** 카드 본문 미리보기. 메모·프로젝트 카드가 같이 쓴다. 넘치는 줄은 가리고 공통 펼침 버튼을 두며, 비었으면 안내 문구를 둔다. */
export function CardPreview({ id, label, rows, empty, onGrow, onToggleTask }: { id: string; label: string; rows: Rows; empty: boolean; onGrow: (px: number) => void; onToggleTask: (sourceLine: number) => void }) {
  const preview = useCollapsible<HTMLUListElement>((list) => previewRowsHeight(list, COLLAPSED_ROWS), [rows]);
  const { expanded } = preview;
  // 카드는 목록과 같은 곡선으로 늘어난 높이만큼 자라고 줄어든다(canvas.css `--preview-extra`).
  const grow = expanded ? preview.extra : 0;
  useEffect(() => onGrow(grow), [grow, onGrow]);
  if (!rows.length) return empty ? <p>내용을 적어 보세요.</p> : null;
  return <>
    <PreviewList ref={preview.ref} id={id} className={`canvas-checklist-preview ${preview.className}`} rows={rows} visibleRows={expanded ? rows.length : COLLAPSED_ROWS} onToggleTask={onToggleTask}/>
    {/* biome-ignore lint/a11y/noStaticElementInteractions: 펼침 버튼 누름이 카드 끌기·더블클릭 열기로 번지지 않게 막기만 한다. */}
    {preview.needsReveal && <div className="canvas-preview-reveal" onPointerDown={stop} onDoubleClick={stop}>
      <Disclosure expanded={expanded} onToggle={preview.toggle} targetId={id} label={`${label} 본문`}/>
    </div>}
  </>;
}
