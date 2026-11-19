import { useEffect, useLayoutEffect } from 'react';
import { PreviewDisclosure, usePreviewDisclosure } from '../../../../shared/markdown/preview/PreviewDisclosure';
import type { PreviewItem } from '../../../../shared/markdown/preview/model';
import { PreviewList } from '../../../../shared/markdown/preview/PreviewList';

type Rows = readonly PreviewItem[];

/** 접힌 카드에 보일 줄 수. 넘치는 줄은 페이드로 가리고 펼침 버튼으로 연다. */
const COLLAPSED_ROWS = 3;

/** 카드 본문 미리보기. 메모·프로젝트 카드가 같이 쓴다. 넘치는 줄은 가리고 공통 펼침 버튼을 두며, 비었으면 안내 문구를 둔다. */
export function CardPreview({ id, label, rows, empty, onGrow, onToggleTask }: { id: string; label: string; rows: Rows; empty: boolean; onGrow: (px: number) => void; onToggleTask: (sourceLine: number) => void }) {
  const preview = usePreviewDisclosure(rows, COLLAPSED_ROWS);
  const { expanded, ref } = preview;
  useLayoutEffect(() => {
    if (!expanded && ref.current) ref.current.scrollTop = 0;
  }, [expanded, ref]);
  // 카드는 목록과 같은 곡선으로 늘어난 높이만큼 자라고 줄어든다(canvas.css `--preview-extra`).
  const grow = expanded ? preview.extra : 0;
  useEffect(() => onGrow(grow), [grow, onGrow]);
  if (!rows.length) return empty ? <p>내용을 적어 보세요.</p> : null;
  return <>
    <PreviewDisclosure label={label} targetId={id} preview={preview}/>
    <PreviewList ref={preview.ref} id={id} className={`canvas-checklist-preview ${preview.className}`} rows={rows} visibleRows={expanded ? rows.length : COLLAPSED_ROWS} onToggleTask={onToggleTask}/>
  </>;
}
