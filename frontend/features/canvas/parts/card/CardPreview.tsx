import type { CSSProperties } from 'react';
import { Icon } from '../../../../shared/Icon';
import type { PreviewItem } from '../../model/preview';

type Rows = readonly PreviewItem[];

/**
 * 본문 미리보기 줄 목록. 편집기와 같은 규칙으로 그린다: 일반 줄은 표식 칸 없이 왼쪽에서 시작하고,
 * 목록 줄은 깊이마다 한 칸(표식 칸 + 간격, CSS `--preview-gutter`)씩 들어간다. 카드·인스펙터가 함께 쓴다.
 */
export function PreviewList({ rows, className }: { rows: Rows; className: string }) {
  return <ul className={className}>
    {/* biome-ignore lint/suspicious/noArrayIndexKey: 미리보기 줄은 본문 순서 그대로의 위치값이라 순서가 곧 정체성이다. */}
    {rows.map((row, index) => <li className={`preview-${row.kind}`} style={{ '--depth': row.depth } as CSSProperties} key={index}>
      {row.kind === 'task' ? <span className={`preview-check${row.done ? ' checked' : ''}`} aria-hidden="true">{row.done ? <Icon name="check" size={12}/> : null}</span>
        : row.kind === 'number' ? <span className="preview-marker">{row.mark}</span>
          : row.kind === 'bullet' ? <span className="preview-marker">•</span> : null}
      <small className={row.done ? 'checked' : ''}>{row.text}</small>
    </li>)}
  </ul>;
}

/** 카드 본문 미리보기. 메모·프로젝트 카드가 같이 쓴다. 잘린 줄 수를 알리고, 비었으면 안내 문구를 둔다. */
export function CardPreview({ rows, remaining, empty }: { rows: Rows; remaining: number; empty: boolean }) {
  if (!rows.length) return empty ? <p>내용을 적어 보세요.</p> : null;
  return <>
    <PreviewList className="canvas-checklist-preview" rows={rows}/>
    {remaining > 0 && <small className="canvas-preview-more">외 {remaining}줄</small>}
  </>;
}
