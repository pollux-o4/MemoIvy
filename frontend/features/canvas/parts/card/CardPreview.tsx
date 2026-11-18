import { Icon } from '../../../../shared/Icon';
import type { PreviewItem } from '../../model/preview';

type Rows = readonly PreviewItem[];

/** 본문 미리보기 줄 목록: 할 일·번호·글머리·일반 줄을 같은 들여쓰기 칸에 맞춘다. 카드·인스펙터가 함께 쓴다. */
export function PreviewList({ rows, className }: { rows: Rows; className: string }) {
  return <ul className={className}>
    {/* biome-ignore lint/suspicious/noArrayIndexKey: 미리보기 줄은 본문 순서 그대로의 위치값이라 순서가 곧 정체성이다. */}
    {rows.map((row, index) => <li className={`preview-${row.kind}`} key={index}>
      {row.kind === 'task' ? <span className={`preview-check${row.done ? ' checked' : ''}`} aria-hidden="true">{row.done ? <Icon name="check" size={12}/> : null}</span>
        : row.kind === 'number' ? <span className="preview-marker">{row.mark}</span>
          : row.kind === 'bullet' ? <span className="preview-marker">•</span> : <span className="preview-spacer"/>}
      <small className={row.done ? 'checked' : ''}>{row.text}</small>
    </li>)}
  </ul>;
}

/** 카드 본문 미리보기. 비었으면 안내 문구를 둔다. */
export function CardPreview({ rows, empty }: { rows: Rows; empty: boolean }) {
  if (!rows.length) return empty ? <p>내용을 적어 보세요.</p> : null;
  return <PreviewList className="canvas-checklist-preview" rows={rows}/>;
}
