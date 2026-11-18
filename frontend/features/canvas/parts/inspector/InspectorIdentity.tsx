import { type RefObject, useState } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import { Icon } from '../../../../shared/Icon';
import { NOTE_COLORS, noteColor } from '../../../../shared/model';
import { canvasTitle } from '../../model/labels';
import { canvasPreview } from '../../model/preview';
import { MarkdownField } from '../../../../shared/markdown/surface/MarkdownField';
import { bindInput } from '../../../../shared/draft/inputBinding';
import { PreviewList } from '../card/CardPreview';
import { useTitledText } from './useTitledText';

/**
 * 1층 · 정체성: 색·이름·(프로젝트면) 소개를 그 자리에서 고친다. 메모는 본문 미리보기만 보이고 본문 편집은 메모 열기로 한다.
 */
export function InspectorIdentity({ note, revision, titleRef, onSaveText, onColor, onClose, onExternal, onOpenAsNote }: {
  note: Note; revision?: number; titleRef: RefObject<HTMLInputElement | null>;
  onSaveText: (text: string, baseRevision?: number) => Promise<number | undefined>; onColor: (color: string) => void; onClose: () => void;
  onExternal: (url: string) => void;
  /** 프로젝트 설명을 전체 메모 편집기로 연다. */
  onOpenAsNote: () => void;
}) {
  const project = Boolean(note.canvas.project);
  const { title, body: intro } = useTitledText(note.text, onSaveText, { version: revision, fallbackTitle: canvasTitle(note) });
  const [palette, setPalette] = useState(false);
  const rows = project ? [] : canvasPreview(note.text, 4).rows;
  return <section className="inspector-identity" aria-label={project ? '프로젝트 정보' : '메모 정보'}>
    <div className="inspector-title-row">
      <button className="inspector-color" style={{ background: noteColor(note.color).accent }} aria-label={`색 바꾸기 · 지금 ${noteColor(note.color).label}`} aria-expanded={palette} title="색 바꾸기" onClick={() => setPalette(!palette)}/>
      <input ref={titleRef} className="inspector-title" aria-label={project ? '프로젝트 이름' : '메모 제목'} placeholder={project ? '프로젝트 이름' : '제목'} title="이름 바꾸기 · F2" {...bindInput(title)}/>
      {project && <button className="inspector-head-action" aria-label="메모로 열기" title="메모로 열기 · Shift+Enter" onClick={onOpenAsNote}><Icon name="note" size={14}/></button>}
      <button className="inspector-head-action" aria-label="선택 해제" title="선택 해제 · Esc" onClick={onClose}><Icon name="close" size={14}/></button>
    </div>
    {palette && <div className="inspector-palette" role="radiogroup" aria-label="메모 색">
      {NOTE_COLORS.map((color) => <button key={color} role="radio" aria-checked={note.color === color} aria-label={noteColor(color).label} title={noteColor(color).label} style={{ background: noteColor(color).accent }} onClick={() => { onColor(color); setPalette(false); }}/>)}
    </div>}
    {project
      ? <MarkdownField className="inspector-intro" draft={intro} label="프로젝트 소개" onExternal={onExternal} placeholder="이 프로젝트를 한두 줄로 소개해 보세요"/>
      : rows.length > 0 && <PreviewList className="inspector-preview" rows={rows}/>}
  </section>;
}
