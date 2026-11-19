import type { CSSProperties, Ref, SyntheticEvent } from 'react';
import type { PreviewItem } from './model';

import './preview_list.css';

type Rows = readonly PreviewItem[];
const stop = (event: SyntheticEvent) => event.stopPropagation();

/**
 * 본문 미리보기 줄 목록. 편집기와 같은 규칙으로 그린다: 일반 줄은 왼쪽에서 시작하고 목록 줄은 깊이마다 한 칸씩 들어간다.
 */
export function PreviewList({ rows, className, id, ref, onToggleTask, visibleRows = rows.length }: { rows: Rows; className: string; id?: string; ref?: Ref<HTMLUListElement>; onToggleTask: (sourceLine: number) => void; visibleRows?: number }) {
  return <ul ref={ref} id={id} className={`preview-list ${className}`}>
    {/* biome-ignore lint/suspicious/noArrayIndexKey: 미리보기 줄은 본문 순서 그대로의 위치값이라 순서가 곧 정체성이다. */}
    {rows.map((row, index) => <li className={`preview-${row.kind}`} style={{ '--depth': row.depth } as CSSProperties} key={index} inert={index >= visibleRows}>
      {row.kind === 'task' && row.sourceLine !== undefined ? <TaskCheck row={row} sourceLine={row.sourceLine} onToggle={onToggleTask}/>
        : row.kind === 'number' ? <span className="preview-marker">{row.mark}</span>
          : row.kind === 'bullet' ? <span className="preview-marker">•</span> : null}
      <small className={row.kind === 'task' && row.task === 'x' ? 'checked' : ''}>{row.text}</small>
    </li>)}
  </ul>;
}

function TaskCheck({ row, sourceLine, onToggle }: { row: Extract<PreviewItem, { kind: 'task' }>; sourceLine: number; onToggle: (line: number) => void }) {
  return <button type="button" role="checkbox" className={`md-checkbox preview-check${row.task === 'x' ? ' is-checked' : row.task === '/' ? ' is-doing' : ''}`}
    aria-checked={row.task === '/' ? 'mixed' : row.task === 'x'} aria-label={row.text || '할 일'}
    onPointerDown={stop} onDoubleClick={stop} onKeyDown={(event) => { if (event.key === 'Enter' || event.key === ' ') stop(event); }}
    onClick={(event) => { event.stopPropagation(); onToggle(sourceLine); }}/>;
}
