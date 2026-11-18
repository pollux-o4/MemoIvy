import { type RefObject, useState } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import { Icon } from '../../../../shared/Icon';
import { joinTitle, NOTE_COLORS, noteColor, splitTitle } from '../../../../shared/model';
import { canvasPreview } from '../../model/preview';
import { useDraftField } from './useDraftField';

/**
 * 1층 · 정체성: 색·이름·(프로젝트면) 소개를 그 자리에서 고친다. 메모는 본문 미리보기만 보이고 본문 편집은 메모 열기로 한다.
 */
export function InspectorIdentity({ note, titleRef, onSaveText, onColor, onClose }: {
  note: Note; titleRef: RefObject<HTMLInputElement | null>;
  onSaveText: (text: string) => void; onColor: (color: string) => void; onClose: () => void;
}) {
  const project = Boolean(note.canvas.project);
  const parts = splitTitle(note.text);
  const title = useDraftField(parts.title, (value) => onSaveText(joinTitle({ ...parts, title: value || parts.title })));
  const intro = useDraftField(parts.body, (value) => onSaveText(joinTitle({ ...parts, body: value })), true);
  const [palette, setPalette] = useState(false);
  const rows = project ? [] : canvasPreview(note.text, 4).rows;
  return <section className="inspector-identity" aria-label={project ? '프로젝트 정보' : '메모 정보'}>
    <div className="inspector-title-row">
      <button className="inspector-color" style={{ background: noteColor(note.color).accent }} aria-label={`색 바꾸기 · 지금 ${noteColor(note.color).label}`} aria-expanded={palette} title="색 바꾸기" onClick={() => setPalette(!palette)}/>
      <input ref={titleRef} className="inspector-title" aria-label={project ? '프로젝트 이름' : '메모 제목'} placeholder={project ? '프로젝트 이름' : '제목'} title="이름 바꾸기 · F2" {...title}/>
      <button className="inspector-close" aria-label="선택 해제" title="선택 해제 · Esc" onClick={onClose}><Icon name="close" size={14}/></button>
    </div>
    {palette && <div className="inspector-palette" role="radiogroup" aria-label="메모 색">
      {NOTE_COLORS.map((color) => <button key={color} role="radio" aria-checked={note.color === color} aria-label={noteColor(color).label} title={noteColor(color).label} style={{ background: noteColor(color).accent }} onClick={() => { onColor(color); setPalette(false); }}/>)}
    </div>}
    {project
      ? <textarea className="inspector-intro" aria-label="프로젝트 소개" placeholder="이 프로젝트를 한두 줄로 소개해 보세요" rows={3} {...intro}/>
      : rows.length > 0 && <div className="inspector-preview">{rows.map((row, index) =>
        // 미리보기 줄은 순서 자체가 정체성이다.
        // biome-ignore lint/suspicious/noArrayIndexKey: 순서가 키
        <div className={`inspector-preview-row preview-${row.kind}`} key={index}>
          <span className={row.done ? 'checked' : ''}>{row.kind === 'task' && row.done ? <Icon name="check" size={12}/> : row.kind === 'number' ? row.mark : row.kind === 'bullet' ? '•' : null}</span><small className={row.done ? 'checked' : ''}>{row.text}</small>
        </div>)}</div>}
  </section>;
}
