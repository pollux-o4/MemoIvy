import { Icon } from '../../../../shared/Icon';
import type { PreviewItem } from '../../model/preview';

type Rows = readonly PreviewItem[];

/** 카드 본문 미리보기: 할 일·번호·글머리·일반 줄을 같은 들여쓰기 칸에 맞춘다. */
export function CardPreview({ rows, empty }: { rows: Rows; empty: boolean }) {
  if (!rows.length) return empty ? <p>내용을 적어 보세요.</p> : null;
  return <ul className="canvas-checklist-preview">
    {/* biome-ignore lint/suspicious/noArrayIndexKey: 미리보기 줄은 본문 순서 그대로의 위치값이라 순서가 곧 정체성이다. */}
    {rows.map((row, index) => <li className={`preview-${row.kind}`} key={index}>
      {row.kind === 'task' ? <span className={row.done ? 'checked' : ''} aria-hidden="true">{row.done ? <Icon name="check" size={12}/> : null}</span>
        : row.kind === 'number' ? <span className="preview-marker">{row.mark}</span>
          : row.kind === 'bullet' ? <span className="preview-marker">•</span> : <span className="preview-spacer"/>}
      <small className={row.done ? 'checked' : ''}>{row.text}</small>
    </li>)}
  </ul>;
}
